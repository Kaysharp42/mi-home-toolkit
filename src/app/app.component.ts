import { Component, inject, ViewChild } from '@angular/core'
import { RouterModule } from '@angular/router'
import { ConfigService } from './config.service'
import { ClosePreferenceDialogComponent } from './dialogs/close-preference-dialog/close-preference-dialog.component'
import { SettingsDialogComponent } from './dialogs/settings-dialog/settings-dialog.component'
import { listen } from '@tauri-apps/api/event'

@Component({
  selector: 'app-root',
  styles: `
    :host {
      display: block;
    }
  `,
  imports: [RouterModule, ClosePreferenceDialogComponent, SettingsDialogComponent],
  templateUrl: './app.component.html',
})
export class AppComponent {
  @ViewChild(ClosePreferenceDialogComponent) closePreferenceDialog!: ClosePreferenceDialogComponent
  
  configService = inject(ConfigService)

  constructor() {
    this.configService.systemTheme$.subscribe((systemTheme) => {
      document
        .getElementsByTagName('html')[0]
        .setAttribute('data-theme', systemTheme)
    })

    // Listen for the show-close-preference-dialog event from Tauri
    this.setupTauriEventListeners()
  }

  private async setupTauriEventListeners() {
    try {
      await listen('show-close-preference-dialog', () => {
        this.closePreferenceDialog?.show()
      })
    } catch (error) {
      console.error('Failed to setup Tauri event listeners:', error)
    }
  }
}
