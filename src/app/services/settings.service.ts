import { Injectable } from '@angular/core'
import { invoke } from '@tauri-apps/api/core'

export interface AppSettings {
  close_to_tray?: boolean
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
}
