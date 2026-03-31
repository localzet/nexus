# Security Policy

## Reporting a Vulnerability

**Do not** open a public issue for security vulnerabilities. Instead, please report security issues to:

📧 **creator@localzet.com**

Please include:
1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Your contact information (optional but helpful)

## Security Considerations

### AGPL v3 License

This project is licensed under GNU Affero General Public License v3.0. When using this software:

- **Network Use Requirement**: If you modify and use this software over a network, you must make the modified source code available to users.
- **Distribution Requirement**: If you distribute this software, you must include the source code.
- **Derivative Works**: Any derivative works must also use the same license.

### Cryptographic Components

NexusDB uses:
- **SHA2** — for hashing
- **BLAKE3** — for checksums
- **Base64** — for encoding

These are standard Rust crate versions and are regularly updated.

## Secure Practices

When deploying NexusDB:

1. **Authentication** — Implement proper authentication mechanisms
2. **Network Security** — Use TLS/SSL for network communication
3. **Access Control** — Restrict database access to authorized users
4. **Backup** — Regular backups are essential
5. **Monitoring** — Enable monitoring and audit logging
6. **Updates** — Keep NexusDB and dependencies updated

## Vulnerability Response

We take security seriously and will:

1. Acknowledge receipt of your report within 24 hours
2. Investigate the vulnerability
3. Develop and test a fix
4. Release a patched version
5. Credit you for the discovery (if desired)

## Security Updates

Follow this repository for security updates or subscribe to release notifications.

---

Thank you for helping keep NexusDB secure!
