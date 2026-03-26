# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | No — please update to the latest version |

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

Instead, report vulnerabilities privately via one of these methods:

1. **GitHub Security Advisories** (preferred): Go to the [Security tab](https://github.com/ponack/linux-claude-desktop/security/advisories) and click "Report a vulnerability"
2. **Email**: Contact the maintainer directly through their GitHub profile

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

### What to expect

- Acknowledgment within 48 hours
- A fix timeline based on severity
- Credit in the release notes (unless you prefer anonymity)

## Security Considerations

### API Keys
API keys are stored in a local SQLite database (`~/.local/share/com.linux-claude-desktop.app/ucd.db`). The database relies on operating system file permissions for protection. Keys are never transmitted anywhere other than to the configured API provider.

### Custom Commands
The custom commands feature executes user-configured shell scripts. Only commands you configure yourself are executed — the app does not run arbitrary commands from external sources.

### Auto-Updates
Updates are downloaded from GitHub Releases over HTTPS. Installation uses `pkexec dpkg -i` which prompts for polkit authentication before installing.

### Temp Files
Temporary files (screenshots, restart scripts) use random or PID-based filenames with restrictive permissions to prevent symlink attacks and race conditions.

### Deep Links
The `claude://` URI protocol caps input at 10,000 characters. Deep link text is processed as chat input, not as shell commands.

### DBus Interface
The DBus interface is only accessible on the session bus (same user). It does not expose any methods that execute arbitrary code.
