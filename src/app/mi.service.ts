import { computed, Injectable, resource } from '@angular/core'
import { invoke } from '@tauri-apps/api/core'
import { GetDevicesResponse } from './types'

// Define the SavedCredentials type
interface SavedCredentials {
  username: string
  country: string
}

@Injectable({
  providedIn: 'root',
})
export class MiService {  login(creds: { email: string; password: string; country?: string; should_save_credentials?: boolean }) {
    return invoke('login', { 
      email: creds.email,
      password: creds.password,
      country: creds.country,
      shouldSaveCredentials: creds.should_save_credentials
    })
  }

  logout() {
    return invoke('logout')
  }
  isLoggedIn() {
    return invoke<boolean>('is_logged_in')
  }  tryAutoLogin() {
    return invoke<boolean>('try_auto_login')
  }

  getCurrentUser() {
    return invoke<{ username: string; country: string } | null>('get_current_user')
  }

  isSessionRestored() {
    return invoke<boolean>('is_session_restored')
  }

  setCountry(country: string) {
    return invoke('set_country', { country })
  }

  countries = resource({ defaultValue: [], loader: () => this.getCountries() })
  countryCodeToName = computed(() => new Map(this.countries.value()))
  private getCountries() {
    return invoke<[code: string, name: string][]>('get_countries')
  }

  getDevices() {
    return invoke<GetDevicesResponse>('get_devices')
  }

  getDevice(did: string) {
    return invoke<GetDevicesResponse>('get_device', { did }).then((res) =>
      res.at(0)
    )
  }

  callDevice(data: { did: string; method: string; params?: string | null }) {
    const { did, method } = data
    let { params } = data
    if ([null, ''].includes(params as '')) params = undefined
    return invoke('call_device', { did, method, params })
  }

  getProp({ did, name }: { did: string; name: string | string[] }) {
    const params = JSON.stringify(Array.isArray(name) ? name : [name])
    return this.callDevice({ did, method: 'get_prop', params })
  }

  setProp({ did, name, value }: { did: string; name: string; value: any }) {
    const params = JSON.stringify([name, value])
    return this.callDevice({ did, method: 'set_ps', params })
  }

  getSavedCredentials() {
    return invoke<SavedCredentials | null>('get_saved_credentials')
  }
}
