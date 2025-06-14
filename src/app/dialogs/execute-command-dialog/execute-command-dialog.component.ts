import {
  Component,
  computed,
  effect,
  inject,
  model,
  output,
  signal,
} from '@angular/core'
import { injectMutation, injectQuery } from '@tanstack/angular-query-experimental'
import { MiService } from '../../mi.service'
import { CommonModule } from '@angular/common'
import { DialogDirective } from '../dialog.directive'
import { FormBuilder, FormsModule, ReactiveFormsModule, Validators } from '@angular/forms'

@Component({
  selector: 'app-execute-command-dialog',
  template: ` <dialog class="modal" app-dialog [visible]="visible()">
    <form
      class="modal-box"
      [formGroup]="form"
      (submit)="$event.preventDefault(); executeCommand()"
    >
      <button
        type="button"
        (click)="device.set(null)"
        [disabled]="callDeviceMutation.isPending()"
        class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
      >
        ✕
      </button>      
      <h3 class="font-bold text-lg mb-4">
        {{ device()?.name }} - Execute command
      </h3>      <!-- Saved Commands Dropdown -->
      <div class="mb-4">
        <label class="block text-sm font-medium mb-1">Load Saved Command</label>
        <div class="flex gap-2">          <select 
            class="select select-bordered flex-1"
            [ngModel]="selectedCommand()"
            (ngModelChange)="loadSavedCommand($event)"
            [ngModelOptions]="{ standalone: true }"
            [disabled]="callDeviceMutation.isPending()"
          >
            <option value="">Select a saved command...</option>
            <option 
              *ngFor="let cmd of savedCommandsQuery.data()" 
              [value]="cmd.name"
            >
              {{ cmd.name }}
            </option>
          </select>
          @if (selectedCommand()) {
            <button
              type="button"
              class="btn btn-error btn-sm"
              (click)="deleteSelectedCommand()"
              [disabled]="deleteCommandMutation.isPending()"
              title="Delete selected command"
            >
              @if (deleteCommandMutation.isPending()) {
                <span class="loading loading-spinner loading-xs"></span>
              } @else {
                ✕
              }
            </button>
          }
        </div>
      </div>

      <div class="flex flex-col gap-2 items-stretch">
        <input
          type="text"
          placeholder="Method"
          spellcheck="false"
          [formControlName]="'method'"
          class="input w-full"
        />

        <textarea
          class="textarea w-full"
          [formControlName]="'params'"
          spellcheck="false"
          autocorrect="off"
          placeholder="Params"
        ></textarea>

        <textarea
          [readonly]="true"
          class="textarea w-full"
          [ngClass]="{
            'textarea-error': callDeviceMutation.isError(),
            'textarea-success': callDeviceMutation.isSuccess(),
          }"
          placeholder="Result"
          spellcheck="false"
          autocorrect="off"
          [ngModel]="form.controls.result.value"
          [ngModelOptions]="{ standalone: true }">
        </textarea>
      </div>

      <!-- Command Name Input for Saving -->
      <div class="mt-4">
        <label class="block text-sm font-medium mb-1">Save Command As</label>
        <input
          type="text"
          placeholder="Enter command name..."
          spellcheck="false"
          [formControlName]="'commandName'"
          class="input input-bordered w-full"
        />
      </div>      <!-- Shortcut Input for Saving -->
      <div class="mt-4">
        <label class="block text-sm font-medium mb-1">Keyboard Shortcut (Optional)</label>        <input
          type="text"
          placeholder="Press key combination..."
          spellcheck="false"
          readonly
          [value]="shortcutDisplayValue()"
          class="input input-bordered w-full"
          [class.input-primary]="isCapturingShortcut()"
          (keydown)="onShortcutKeyDown($event)"
          (focus)="startShortcutCapture()"
          (blur)="stopShortcutCapture(); validateShortcut()"
        />
        @if (shortcutError()) {
          <div class="text-error text-sm mt-1">{{ shortcutError() }}</div>
        }
        @if (isCapturingShortcut()) {
          <div class="text-primary text-sm mt-1">
            Press your desired key combination...
          </div>
        } @else {
          <div class="text-sm text-gray-500 mt-1">
            Global shortcut that works even when app is minimized
          </div>
        }
      </div>

      <div class="flex gap-2 mt-4">
        <button
          class="btn btn-primary flex-1"
          type="submit"
          [disabled]="callDeviceMutation.isPending()"
        >
          @if (callDeviceMutation.isPending()) {
            <span class="loading loading-spinner loading-sm"></span>
            Executing...
          } @else {
            Execute
          }
        </button>
          <button
          type="button"
          class="btn btn-secondary"
          (click)="saveCommand()"
          [disabled]="!canSaveCommand() || saveCommandMutation.isPending()"
        >
          @if (saveCommandMutation.isPending()) {
            <span class="loading loading-spinner loading-sm"></span>
          } @else {
            {{ isUpdatingExistingCommand() ? 'Update' : 'Save' }}
          }
        </button>
      </div>

      @if (saveCommandMutation.isError() && showErrorMessage()) {
        <div class="alert alert-error mt-2">
          <span>{{ saveCommandMutation.error() }}</span>
        </div>
      }

      @if (saveCommandMutation.isSuccess() && showSuccessMessage()) {
        <div class="alert alert-success mt-2">
          <span>{{ isUpdatingExistingCommand() ? 'Command updated successfully!' : 'Command saved successfully!' }}</span>
        </div>
      }
    </form>
  </dialog>`,
  styles: [``],
  imports: [CommonModule, DialogDirective, FormsModule, ReactiveFormsModule],
})
export class ExecuteCommandDialogComponent {
  fb = inject(FormBuilder)
  device = model<{ did: number | string; name: string } | null>(null)
  did = computed(() => this.device()?.did)
  visible = computed(() => !!this.device())
  success = output()
  showSuccessMessage = signal(false)
  showErrorMessage = signal(false)
  selectedCommand = signal<string>('')
  shortcutError = signal<string>('')
  isCapturingShortcut = signal<boolean>(false)
  shortcutDisplayValue = signal<string>('')

  miService = inject(MiService)
  form = this.fb.group({
    method: '',
    params: '',
    result: '' as any,
    commandName: ['', Validators.required],
    shortcut: '',
  })

  // Query to load saved commands
  savedCommandsQuery = injectQuery(() => ({
    queryKey: ['saved-commands'],
    queryFn: () => this.miService.getSavedCommands(),
    staleTime: 1000 * 60 * 5, // 5 minutes
  }))  // Mutation to save commands
  saveCommandMutation = injectMutation(() => ({
    mutationFn: (data: { name: string; method: string; params: string; shortcut?: string; update?: boolean }) =>
      data.update 
        ? this.miService.updateCommand(data.name, data.method, data.params, data.shortcut)
        : this.miService.saveCommand(data.name, data.method, data.params, data.shortcut),
    onSuccess: (_, variables) => {
      this.savedCommandsQuery.refetch()
      
      // Only clear form if it's a new save, not an update
      if (!variables.update) {
        this.form.patchValue({ commandName: '' })
        this.selectedCommand.set('')
      }
      // For updates, keep the current selection
      
      this.showSuccessMessage.set(true)
      this.showErrorMessage.set(false)
      // Hide success message after 5 seconds
      setTimeout(() => {
        this.showSuccessMessage.set(false)
      }, 5000)
    },
    onError: () => {
      this.showErrorMessage.set(true)
      // Hide error message after 5 seconds
      setTimeout(() => {
        this.showErrorMessage.set(false)
      }, 5000)
    },
  }))

  // Mutation to delete commands
  deleteCommandMutation = injectMutation(() => ({
    mutationFn: (name: string) => this.miService.deleteCommand(name),
    onSuccess: () => {
      this.savedCommandsQuery.refetch()
      this.selectedCommand.set('')
      this.form.patchValue({ commandName: '' })
    },
  }))
  private visibleEffect = effect(() => {
    if (this.visible()) {
      this.callDeviceMutation.reset()
      this.saveCommandMutation.reset()
      this.deleteCommandMutation.reset()
      this.showSuccessMessage.set(false)
      this.showErrorMessage.set(false)
      this.selectedCommand.set('')
      this.shortcutDisplayValue.set('')
    }
  })

  openCloseEffect = effect(() => {
    if (this.visible()) {
      this.form.reset()
      this.shortcutDisplayValue.set('')
    }
  })

  callDeviceMutation = injectMutation(() => ({
    mutationFn: (data: {
      did: string
      method: string
      params?: string | null
    }) => this.miService.callDevice(data),
    onSuccess: () => this.success.emit(),
  }))

  callDeviceResultEffect = effect(() => {
    const data = this.callDeviceMutation.data()
    const error = this.callDeviceMutation.error()
    const isError = this.callDeviceMutation.isError()
    const isPending = this.callDeviceMutation.isPending()

    const result = this.form.controls.result

    if (isPending) return result.setValue('Loading...')
    if (isError) return result.setValue(error || 'Error')
    return result.setValue(JSON.stringify(data))
  })
  executeCommand() {
    if (this.callDeviceMutation.isPending()) return
    const did = this.did()?.toString()
    const { method, params } = this.form.value
    if (!did || !method) return
    this.callDeviceMutation.mutate({ did, method, params })
  }  saveCommand() {
    if (this.saveCommandMutation.isPending()) return
    const { method, params, commandName, shortcut } = this.form.value
    if (!method || !commandName) return
    
    // Check if we're updating an existing command
    const savedCommands = this.savedCommandsQuery.data()
    const existingCommand = savedCommands?.find(cmd => cmd.name === commandName)
    const isUpdate = !!existingCommand
    
    this.saveCommandMutation.mutate({ 
      name: commandName, 
      method, 
      params: params || '',
      shortcut: shortcut || undefined,
      update: isUpdate
    })
  }  loadSavedCommand(commandName: string) {
    this.selectedCommand.set(commandName)
    if (!commandName) return
    const savedCommands = this.savedCommandsQuery.data()
    const command = savedCommands?.find(cmd => cmd.name === commandName)
    if (command) {
      this.form.patchValue({
        method: command.method,
        params: command.params,
        commandName: command.name,
        shortcut: command.shortcut || ''
      })
      // Sync the display value with the form control
      this.shortcutDisplayValue.set(command.shortcut || '')
    }
  }

  deleteSelectedCommand() {
    const commandName = this.selectedCommand()
    if (!commandName || this.deleteCommandMutation.isPending()) return
    this.deleteCommandMutation.mutate(commandName)
  }
  canSaveCommand() {
    const { method, commandName } = this.form.value
    return !!(method && commandName && this.form.get('commandName')?.valid)
  }

  isUpdatingExistingCommand() {
    const { commandName } = this.form.value
    if (!commandName) return false
    const savedCommands = this.savedCommandsQuery.data()
    return savedCommands?.some(cmd => cmd.name === commandName) || false
  }

  validateShortcut() {
    const shortcut = this.form.get('shortcut')?.value
    if (!shortcut) {
      this.shortcutError.set('')
      return
    }

    this.miService.validateShortcut(shortcut).then(
      () => {
        this.shortcutError.set('')
      }
    ).catch(
      (error) => {
        this.shortcutError.set(error || 'Invalid shortcut format')
      }
    )
  }  startShortcutCapture() {
    this.isCapturingShortcut.set(true)
    this.shortcutError.set('')
    // Clear both the form control and display value when starting to capture
    this.form.patchValue({ shortcut: '' })
    this.shortcutDisplayValue.set('')
  }
  stopShortcutCapture() {
    this.isCapturingShortcut.set(false)
  }

  // Map KeyboardEvent.code to the appropriate key name for shortcuts
  private mapEventCodeToKeyName(code: string): string {
    // Handle numpad keys specifically
    if (code.startsWith('Numpad')) {
      return code // Use the full code like "Numpad8", "NumpadAdd", etc.
    }
    
    // Handle regular digit keys
    if (code.startsWith('Digit')) {
      return code.slice(5) // "Digit8" -> "8"
    }
    
    // Handle regular letter keys
    if (code.startsWith('Key')) {
      return code.slice(3) // "KeyA" -> "A"
    }
    
    // Handle special cases
    switch (code) {
      case 'Space': return 'Space'
      case 'Enter': return 'Enter'
      case 'Escape': return 'Escape'
      case 'Backspace': return 'Backspace'
      case 'Tab': return 'Tab'
      case 'Delete': return 'Delete'
      case 'Insert': return 'Insert'
      case 'Home': return 'Home'
      case 'End': return 'End'
      case 'PageUp': return 'PageUp'
      case 'PageDown': return 'PageDown'
      case 'ArrowLeft': return 'ArrowLeft'
      case 'ArrowRight': return 'ArrowRight'
      case 'ArrowUp': return 'ArrowUp'
      case 'ArrowDown': return 'ArrowDown'
      case 'F1': case 'F2': case 'F3': case 'F4': case 'F5': case 'F6':
      case 'F7': case 'F8': case 'F9': case 'F10': case 'F11': case 'F12':
        return code
      case 'Minus': return '-'
      case 'Equal': return '='
      case 'BracketLeft': return '['
      case 'BracketRight': return ']'
      case 'Backslash': return '\\'
      case 'Semicolon': return ';'
      case 'Quote': return '\''
      case 'Comma': return ','
      case 'Period': return '.'
      case 'Slash': return '/'
      case 'Backquote': return '`'
      default:
        // For any other codes, just return the code as-is
        return code
    }
  }  onShortcutKeyDown(event: KeyboardEvent) {
    if (!this.isCapturingShortcut()) return

    // Prevent the default behavior to stop Unicode characters from being inserted
    event.preventDefault()
    event.stopPropagation()

    const keys: string[] = []
    
    // Add modifier keys in the correct order
    if (event.ctrlKey) keys.push('Ctrl')
    if (event.altKey) keys.push('Alt')
    if (event.shiftKey) keys.push('Shift')
    if (event.metaKey) keys.push('Meta')

    // Add the main key if it's not a modifier key
    if (!['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'].includes(event.code)) {
      // Use event.code to distinguish between numpad and regular keys
      let keyName = this.mapEventCodeToKeyName(event.code)
      
      keys.push(keyName)
      
      // Set both the form control and display value
      const shortcut = keys.join('+')
      this.form.get('shortcut')?.setValue(shortcut)
      this.form.get('shortcut')?.markAsDirty()
      this.shortcutDisplayValue.set(shortcut)
      
      // Stop capturing after a complete shortcut is entered
      this.stopShortcutCapture()
    } else if (keys.length > 0) {
      // Show current modifier keys being held
      const shortcut = keys.join('+') + '+'
      this.shortcutDisplayValue.set(shortcut)
    }
  }
}
