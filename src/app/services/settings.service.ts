import { Injectable } from '@angular/core'
import { invoke } from '@tauri-apps/api/core'

export interface AppSettings {
  close_to_tray?: boolean
  auto_start?: boolean
  auto_hide_to_tray?: boolean
}

@Injectable({
  providedIn: 'root'
})
export class SettingsService {
  
  async getAppSettings(): Promise<AppSettings> {
    return invoke('get_app_settings')
  }

  async saveCloseToTrayPreference(closeToTray: boolean): Promise<void> {
    return invoke('save_close_to_tray_preference', { closeToTray })
  }

  async saveAutoStartPreference(autoStart: boolean): Promise<void> {
    return invoke('save_auto_start_preference', { autoStart })
  }

  async saveAutoHidePreference(autoHide: boolean): Promise<void> {
    return invoke('save_auto_hide_preference', { autoHide })
  }

  async saveAllSettings(settings: {
    closeToTray?: boolean
    autoStart?: boolean
    autoHideToTray?: boolean
  }): Promise<void> {
    return invoke('save_all_settings', {
      closeToTray: settings.closeToTray,
      autoStart: settings.autoStart,
      autoHideToTray: settings.autoHideToTray
    })
  }
}
