import { AUDIT_CONFIG } from './config';

export class PIITokenizer {
  private tokenMap: Map<string, string> = new Map();
  private tokenIndex: number = 0;

  tokenizePII(value: string, type: 'email' | 'phone' | 'ssn' | 'custom'): string {
    if (!AUDIT_CONFIG.privacy.tokenizePII) {
      return this._maskValue(value, type);
    }

    // Check if already tokenized
    if (this.tokenMap.has(value)) {
      return this.tokenMap.get(value)!;
    }

    // Generate token
    const token = `pii_${type}_${++this.tokenIndex}`;
    this.tokenMap.set(value, token);
    return token;
  }

  detokenizePII(token: string): string | null {
    // Reverse lookup (for authorized access only)
    for (const [value, t] of this.tokenMap.entries()) {
      if (t === token) {
        return value;
      }
    }
    return null;
  }

  private _maskValue(value: string, type: string): string {
    switch (type) {
      case 'email':
        if (AUDIT_CONFIG.privacy.maskEmails) {
          return value.replace(/(.{2})(.*)(@.*)/, '$1***$3');
        }
        break;
      case 'phone':
        if (AUDIT_CONFIG.privacy.maskPhones) {
          return value.replace(/(\d{3}).*?(\d{4})/, '$1***$2');
        }
        break;
      case 'ssn':
        return value.replace(/(\d{3})-?/, '$1--') + 'xxxx';
      default:
        return `[MASKED_${type.toUpperCase()}]`;
    }
    return value;
  }

  // Mask email addresses
  maskEmail(email: string): string {
    return this._maskValue(email, 'email');
  }

  // Mask phone numbers
  maskPhone(phone: string): string {
    return this._maskValue(phone, 'phone');
  }

  // Mask SSN
  maskSSN(ssn: string): string {
    return this._maskValue(ssn, 'ssn');
  }

  // Clear token map (for security)
  clearTokens(): void {
    this.tokenMap.clear();
    this.tokenIndex = 0;
  }
}
