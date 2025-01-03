import { writable } from 'svelte/store';

export interface GameConfig {
  username: string;
  password: string;
  otp: string;
  gamePath: string;
  isSteam: boolean;
  language: number;
  dx11: boolean;
  expansionLevel: number;
  region: number;
  // Added new fields
  isFreeTrial: boolean;
  dalamudEnabled: boolean;
  dpiAwareness: 'Aware' | 'Unaware';
  additionalLaunchArgs: string;
  encryptArguments: boolean;
  savedLogin: boolean;
  autoLogin: boolean;
  directXVersion: '11' | '9';
  clientLanguage: 'Japanese' | 'English' | 'German' | 'French';
}

const initialConfig: GameConfig = {
  username: '',
  password: '',
  otp: '',
  gamePath: '',
  isSteam: false,
  language: 1,
  dx11: true,
  expansionLevel: 4,
  region: 3,
  isFreeTrial: false,
  dalamudEnabled: true,
  dpiAwareness: 'Aware',
  additionalLaunchArgs: '',
  encryptArguments: true,
  savedLogin: false,
  autoLogin: false,
  directXVersion: '11',
  clientLanguage: 'English'
};

export const gameConfig = writable<GameConfig>(initialConfig);
