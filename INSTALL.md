# Install BongTerm

`v0.1.0-mvp0` is not published yet. These instructions define the release verification flow for signed MSIX artifacts.

## Verify Signature

```powershell
signtool verify /pa .\BongTerm-0.1.0-mvp0-x64.msix
```

## Verify Checksum

```powershell
Get-FileHash .\BongTerm-0.1.0-mvp0-x64.msix -Algorithm SHA256
Get-Content .\BongTerm-0.1.0-mvp0-x64.msix.sha256
```

The hash from `Get-FileHash` must match the `.sha256` file and `checksums.txt`.

## Install

```powershell
Add-AppxPackage .\BongTerm-0.1.0-mvp0-x64.msix
```

## Uninstall

```powershell
Get-AppxPackage *BongTerm* | Remove-AppxPackage
```

## SmartScreen

New OV-signed Windows apps may show SmartScreen reputation warnings until enough installs build reputation. Verify the signature and checksums before choosing to continue.
