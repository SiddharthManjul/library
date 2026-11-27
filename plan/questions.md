# Open questions for the founder

Agents do not decide product questions. Each entry says what is asked, the options, and what each costs.

## Microsoft Store identity
Need: a Partner Center account (no registration fee) and the app name reservation, which gives the real package identity name and publisher string for `electron-builder.yml`. Until then the appx uses a placeholder and cannot be submitted.
Cost of waiting: none for development; blocks the first Store submission.

## Package format name
electron-builder builds `.appx`, not `.msix`. The Store accepts it and signs it. If MSIX-only features are needed later (for example app installer auto-update outside the Store), we move to a different packager.

## Windows sign-in from a packaged app
The OAuth loopback redirect (P0-16) opens the system browser and listens on 127.0.0.1 from the node. Some corporate machines block local listening ports. Option A: loopback (planned). Option B: a custom URL scheme registered by the package. B needs the packaged build to register the protocol; A needs no registration but may be blocked.

## Theme control
Ice Dark is the default. A theme switcher is not in the designs. Today `?theme=` and a saved value switch themes in development only. Decide where the switcher lives (Settings, account menu) when the Settings screens are designed.

## Brand kit inconsistencies
Listed in the README (Interface and screens). They are corrected as each screen is built; no decision needed unless you want any of them changed.
