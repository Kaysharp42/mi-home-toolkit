import { CommonModule } from '@angular/common'
import { Component, inject, signal } from '@angular/core'
import { DialogDirective } from '../dialog.directive'
import { SettingsService } from '../../services/settings.service'
import { WebviewWindow, getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

@Component({
  selector: 'app-close-preference-dialog',
  template: `
    <dialog class="modal" app-dialog [visible]="visible()">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">Close to System Tray</h3>
        
        <div class="mb-6">
          <p class="text-sm text-gray-600 mb-4">
            Would you like the application to minimize to the system tray when you close it, 
            or close completely?
          </p>
          
          <div class="flex flex-col gap-3">
            <label class="cursor-pointer label justify-start gap-3">
              <input 
                type="radio" 
                name="closePreference" 
                class="radio radio-primary" 
                [checked]="selectedOption() === 'tray'"
                (change)="selectedOption.set('tray')"
              />
              <span class="label-text">
                <span class="font-medium">Minimize to System Tray</span>
                <span class="block text-xs text-gray-500">
                  The app will continue running in the background and can be accessed from the system tray
                </span>
              </span>
            </label>
            
            <label class="cursor-pointer label justify-start gap-3">
              <input 
                type="radio" 
                name="closePreference" 
                class="radio radio-primary" 
                [checked]="selectedOption() === 'close'"
                (change)="selectedOption.set('close')"
              />
              <span class="label-text">
                <span class="font-medium">Close Application</span>
                <span class="block text-xs text-gray-500">
                  The app will close completely and stop running
                </span>
              </span>
            </label>
          </div>
        </div>
        
        <div class="modal-action">
          <button 
            class="btn btn-primary"
            [disabled]="!selectedOption() || isLoading()"
            (click)="savePreference()"
          >
            @if (isLoading()) {
              <span class="loading loading-spinner loading-sm"></span>
            }
            Save Preference
          </button>
        </div>
      </div>
    </dialog>
  `,
  styles: [``],
  imports: [CommonModule, DialogDirective],
  standalone: true,
})
export class ClosePreferenceDialogComponent {
  visible = signal(false)
    selectedOption = signal<'tray' | 'close' | null>(null)
  isLoading = signal(false)
  
  settingsService = inject(SettingsService);
  
  async savePreference() {
    const option = this.selectedOption()
    if (!option) return

    this.isLoading.set(true)
    try {
      const closeToTray = option === 'tray'
      await this.settingsService.saveCloseToTrayPreference(closeToTray)
      this.visible.set(false)
      this.selectedOption.set(null)
      
      // If user chose to close to tray, hide the main window immediately
      if (closeToTray) {
        console.log('User chose to close to tray, attempting to hide main window...')
        
        // Get current window first
        const currentWindow = getCurrentWebviewWindow()
        console.log('Current window label:', currentWindow.label)
        
        // List all available windows for debugging
        const allWindows = await WebviewWindow.getAll()
        console.log('All available windows:', allWindows.map(w => w.label))
        
        // Try multiple approaches to get the main window
        let mainWindow = await WebviewWindow.getByLabel('main')
        if (!mainWindow) {
          // If 'main' doesn't work, try the current window (since the dialog is part of the main window)
          mainWindow = currentWindow
          console.log('Using current window as main window')
        }
        
        console.log('Main window found:', mainWindow)
        if (mainWindow) {
          console.log('Hiding main window...')
          await mainWindow.hide()
          console.log('Main window hidden successfully')
        } else {
          console.error('Main window not found!')
        }
      }
    } catch (error) {
      console.error('Failed to save preference:', error)
    } finally {
      this.isLoading.set(false)
    }
  }

  show() {
    this.visible.set(true)
    this.selectedOption.set(null)
  }
}
