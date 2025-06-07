import { CommonModule } from '@angular/common'
import { Component, computed, effect, inject } from '@angular/core'
import { IconComponent } from '../icon/icon.component'
import {
  FormBuilder,
  FormsModule,
  ReactiveFormsModule,
  Validators,
} from '@angular/forms'
import { AuthService } from '../auth.service'
import { Router } from '@angular/router'
import { injectMutation } from '@tanstack/angular-query-experimental'
import { MiService } from '../mi.service'

@Component({
  template: `<form
    class="flex flex-col gap-2 w-80"
    (ngSubmit)="login($event)"
    [formGroup]="form"
  >
    <label class="input flex items-center gap-2">
      <span class="label"><app-icon icon="email" class="w-4 h-4" /></span>
      <input [formControlName]="'email'" type="text" placeholder="Login" />
    </label>

    <label class="input flex items-center gap-2">
      <span class="label"><app-icon icon="password" class="w-4 h-4" /></span>
      <input
        [formControlName]="'password'"
        type="password"
        placeholder="Password"
      />
    </label>    
    <select class="select" [formControlName]="'country'">
      <option disabled>Server location</option>
      <option *ngFor="let country of countries()" [value]="country[0]">
        {{ country[1] }}
      </option>
      <option disabled>If not listed, try matching from above</option>
    </select>
    
    <div class="form-control">
      <label class="label cursor-pointer justify-start">
        <input type="checkbox" [formControlName]="'saveCredentials'" class="checkbox" />
        <span class="label-text ml-2">Remember me</span>
      </label>
    </div>

    <button
      [disabled]="form.invalid || loading()"
      type="submit"
      class="btn w-full"
    >
      <app-icon *ngIf="!loading()" icon="login" class="w-4 h-4" />
      <span *ngIf="loading()" class="loading loading-spinner loading-xs"></span>
      Login
    </button>

    @if (loginMutation.isError()) {
      <div class="toast toast-center">
        <div role="alert" class="alert alert-error">
          <div></div>
          <div>
            <h3 class="font-bold">Login failed</h3>
            <div class="text-xs">{{ loginMutation.error() }}</div>
          </div>
          <button
            type="button"
            class="btn btn-sm btn-circle btn-ghost"
            (click)="loginMutation.reset()"
          >
            ✕
          </button>
        </div>
      </div>
    }
  </form>`,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100%;
      justify-content: center;
      align-items: center;
    }
  `,
  imports: [CommonModule, IconComponent, FormsModule, ReactiveFormsModule],
})
export class LoginPageComponent {  
  fb = inject(FormBuilder)
  router = inject(Router)
  authService = inject(AuthService)
  miService = inject(MiService);
  loginMutation = injectMutation(() => ({
    mutationFn: (credentials: {
      email: string
      password: string
      country?: string
      should_save_credentials: boolean
    }) => this.authService.login(credentials),
    onSuccess: () => this.router.navigateByUrl('devices'),
  }))
  loading = computed(() => this.loginMutation.isPending())
  countries = this.miService.countries.value
  form = this.fb.nonNullable.group({
    email: this.fb.nonNullable.control('', [Validators.required]),
    password: this.fb.nonNullable.control('', [Validators.required]),
    country: this.fb.nonNullable.control('cn', [Validators.required]),
    saveCredentials: this.fb.nonNullable.control(true)
  })
  constructor() {
    // Try to fill the form with saved credentials if available
    this.miService.getSavedCredentials().then((creds: { username: string; country: string } | null) => {
      if (creds) {
        this.form.patchValue({
          email: creds.username,
          country: creds.country,
          password: '',
          // Keep the "Remember me" checked by default since they previously chose to save
          saveCredentials: true
        });
      }
    }).catch((err: Error) => console.error('Error loading saved credentials:', err));
  }
  disabledFormEffect = effect(() =>
    this.loading() ? this.form.disable() : this.form.enable()
  );  
    async login(event: SubmitEvent) {
    event.preventDefault()
    if (this.form.invalid) return
    const { email, password, country, saveCredentials } = this.form.value
    if (!email || !password) return
    this.loginMutation.mutate({ 
      email, 
      password, 
      country, 
      should_save_credentials: saveCredentials ?? true 
    })
  }
}
