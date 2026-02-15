# Security Policy

## Supported Versions

We actively support and provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **DO NOT** create a public GitHub issue for security vulnerabilities
2. Email: hi@ultrabalancer.com with subject: "Security Vulnerability Report"
3. Include as much detail as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 24-48 hours
- **Initial Assessment**: Within 7 days
- **Fix Timeline**: Depends on severity (see below)

### Severity Classification

| Severity | Response Time | Fix Target |
|----------|---------------|------------|
| Critical | 24 hours      | 7 days     |
| High     | 48 hours      | 14 days    |
| Medium   | 7 days        | 30 days    |
| Low      | 30 days       | Next minor |

## Security Best Practices

When deploying UltraBalancer:

1. **Network Isolation**: Run in a trusted network environment
2. **TLS/SSL**: Use HTTPS for sensitive traffic
3. **Access Control**: Limit admin endpoint access
4. **Monitoring**: Enable metrics and health checks
5. **Updates**: Keep your installation up to date

## Security Features

- Connection rate limiting
- IP-based filtering
- Request timeout handling
- Circuit breaker pattern
- Secure error handling (no sensitive data in logs)

## Attribution

We appreciate responsible disclosure and will credit reporters (if desired) in the security advisory.