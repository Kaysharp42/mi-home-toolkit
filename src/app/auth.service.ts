import { Injectable, inject } from '@angular/core'
import { Router } from '@angular/router'
import { MiService } from './mi.service'
import { BehaviorSubject, map } from 'rxjs'
import { toSignal } from '@angular/core/rxjs-interop'

type User = { email: string; country?: string }

@Injectable({
  providedIn: 'root',
})
export class AuthService {
  miService = inject(MiService)
  router = inject(Router)

  user$ = new BehaviorSubject<User | null>(null)
  user = toSignal(this.user$)
  loggedIn$ = this.user$.pipe(map(Boolean))
  // Track auto-login completion
  autoLoginCompleted$ = new BehaviorSubject<boolean>(false)
  autoLoginCompleted = toSignal(this.autoLoginCompleted$)

  constructor() {
    this.checkForSavedLogin();
  }  private async checkForSavedLogin() {
    try {
      const autoLoginSuccess = await this.miService.tryAutoLogin();
      
      if (autoLoginSuccess) {
        // We successfully restored the session
        const currentUser = await this.miService.getCurrentUser();
        
        if (currentUser) {
          this.user$.next({ 
            email: currentUser.username, 
            country: currentUser.country 
          });
          
          // Navigate to devices page if we're currently on login page
          if (this.router.url === '/login') {
            this.router.navigate(['/devices']);
          }
        }
      }
    } catch (error) {
      console.error('Error checking saved login:', error);
    } finally {
      this.autoLoginCompleted$.next(true);
    }
  }

  async setCountry(country: string) {
    const res = this.miService.setCountry(country)
    this.user$.next({ ...this.user$.value!, country })
    return res
  }

  async login(creds: { email: string; password: string; country?: string; should_save_credentials?: boolean }) {
    const res = await this.miService.login(creds)
    this.user$.next(creds)
    return res
  }
  async logout() {
    await this.miService.logout();
    this.user$.next(null);
    this.router.navigate(['/login']);
  }
}
