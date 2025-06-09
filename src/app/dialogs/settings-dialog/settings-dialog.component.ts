import { CommonModule } from '@angular/common'
import { Component, inject, signal } from '@angular/core'
import { DialogDirective } from '../dialog.directive'
import { SettingsService, AppSettings } from '../../services/settings.service'

@Component({
  selector: 'app-settings-dialog',
  template: `    
  <dialog class="modal" app-dialog [visible]="visible()">
      <div class="modal-box max-w-2xl w-full">
        <h3 class="font-bold text-lg mb-4">Application Settings</h3>
        
        <div class="mb-6">
          <div class="flex flex-col gap-4">
            
            <!-- Close to Tray Setting -->
            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-3">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary" 
                  [checked]="settings().close_to_tray === true"
                  (change)="updateSetting('close_to_tray', $event)"
                />
                <div class="label-text flex flex-col">
                  <span class="font-medium">Minimize to System Tray</span>
                  <span class="text-xs text-gray-500">
                    When closing the app, minimize to system tray instead of closing completely
                  </span>
                </div>
              </label>
            </div>

            <div class="divider my-2"></div>

            <!-- Auto Start Setting -->
            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-3">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary" 
                  [checked]="settings().auto_start === true"
                  (change)="updateSetting('auto_start', $event)"
                />
                <div class="label-text flex flex-col">
                  <span class="font-medium">Start with Windows</span>
                  <span class="text-xs text-gray-500">
                    Automatically start the application when Windows starts
                  </span>
                </div>
              </label>
            </div>

            <!-- Auto Hide Setting -->
            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-3">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-primary" 
                  [checked]="settings().auto_hide_to_tray === true"
                  (change)="updateSetting('auto_hide_to_tray', $event)"
                />
                <div class="label-text flex flex-col">
                  <span class="font-medium">Start Hidden in Tray</span>
                  <span class="text-xs text-gray-500">
                    When the app starts, automatically hide to system tray
                  </span>
                </div>
              </label>
            </div>

          </div>
        </div>
        
        <div class="modal-action">
          <button 
            class="btn btn-ghost"
            (click)="close()"
            [disabled]="isLoading()"
          >
            Cancel
          </button>
          <button 
            class="btn btn-primary"
            (click)="saveSettings()"
            [disabled]="isLoading()"
          >
            @if (isLoading()) {
              <span class="loading loading-spinner loading-sm"></span>
            }
            Save Settings
          </button>
        </div>
      </div>
    </dialog>
  `,
  styles: [``],
  imports: [CommonModule, DialogDirective],
  standalone: true,
})
export class SettingsDialogComponent {
  visible = signal(false)
  settings = signal<AppSettings>({})
  isLoading = signal(false)
  
  settingsService = inject(SettingsService);
  
  async show() {
    try {
      const currentSettings = await this.settingsService.getAppSettings()
      this.settings.set(currentSettings)
      this.visible.set(true)
    } catch (error) {
      console.error('Failed to load settings:', error)
    }
  }

  close() {
    this.visible.set(false)
  }

  updateSetting(key: keyof AppSettings, event: Event) {
    const target = event.target as HTMLInputElement
    const value = target.checked
    
    this.settings.update(current => ({
      ...current,
      [key]: value
    }))
  }

  async saveSettings() {
    this.isLoading.set(true)
    try {
      const settingsToSave = this.settings()
      await this.settingsService.saveAllSettings({
        closeToTray: settingsToSave.close_to_tray,
        autoStart: settingsToSave.auto_start,
        autoHideToTray: settingsToSave.auto_hide_to_tray
      })
      this.close()
    } catch (error) {
      console.error('Failed to save settings:', error)
      // You could add a toast notification here
    } finally {
      this.isLoading.set(false)
    }
  }
}
