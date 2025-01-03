# XIV Loader

A modern, fast and user-friendly FFXIV game launcher built with Tauri, SvelteKit and TypeScript.


## Features

- Smooth UI transitions and animations with View Transitions API
- Dark/Light theme support with animated transitions
- Secure game authentication system
- Plugin management system
- Game integrity verification
- Automatic updates
- Dalamud integration
- Multi-platform support (Windows, Linux)

## Tech Stack

- **Frontend**: 
  - SvelteKit 2.x
  - TypeScript
  - TailwindCSS
  - Lucide Icons
  - Shadcn-Svelte components
  - View Transitions API

- **Backend**: 
  - Rust
  - Tauri 2.0
  - Tokio async runtime
  - SHA1 verification
  - Plugin system

## Development Setup

1. Install dependencies:
   - [Rust](https://rustup.rs/)
   - [Node.js](https://nodejs.org/)
   - [VS Code](https://code.visualstudio.com/)

2. Recommended VS Code extensions:
   - Svelte
   - Tauri
   - rust-analyzer
   - Tailwind CSS IntelliSense

3. Install project dependencies:
```
bash
bun run tauri dev
Building
bun run tauri build

```
##Contributing
Contributions are welcome! Check out our issues page or submit a pull request.

License
Mozilla Public License Version 2.0 (MPL-2.0)

This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/.