# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability, please send an email to the maintainer at mrproject72@gmail.com. Do not report security vulnerabilities through public GitHub issues.

Please include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| older   | :white_check_mark: |

## Security Considerations

mini-eq is an audio processing application that runs with PipeWire access. Security considerations include:

- Audio routing permissions
- PipeWire filter-chain access
- Preset file integrity (avoid loading untrusted APO files)
- D-Bus control interface access

## Handling Security Issues

1. Acknowledge receipt within 48 hours
2. Provide a timeline for resolution
3. Release a patched version as soon as possible
4. Publish a security advisory
