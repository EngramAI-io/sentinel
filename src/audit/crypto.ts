import { createHmac, randomBytes, createCipheriv, createDecipheriv, createHash } from 'crypto';
import { AUDIT_CONFIG } from './config';

export interface EncryptedField {
  encrypted_data: string;  // Base64
  iv: string;              // Base64
  auth_tag: string;        // Base64
  algorithm: string;       // "AES-256-GCM"
}

export class CryptoEngine {
  private hmacKey: Buffer;
  private encryptionKey: Buffer;

  constructor() {
    // HMAC key: from env or generate
    const hmacKeyEnv = process.env.AUDIT_HMAC_KEY || '';
    this.hmacKey = hmacKeyEnv 
      ? Buffer.from(hmacKeyEnv, 'hex')
      : randomBytes(32);

    // Encryption key: from env or generate
    const encKeyEnv = process.env.AUDIT_ENCRYPTION_KEY || '';
    this.encryptionKey = encKeyEnv
      ? Buffer.from(encKeyEnv, 'hex')
      : randomBytes(32);

    if (hmacKeyEnv || encKeyEnv) {
      console.log('[CRYPTO] Initialized with keys from environment');
    } else {
      console.log('[CRYPTO] Initialized with auto-generated keys (not for production)');
    }
  }

  // Generate HMAC signature for integrity
  signLogEntry(entry: Record<string, any>): string {
    const entryStr = JSON.stringify(entry);
    const signature = createHmac('sha256', this.hmacKey)
      .update(entryStr)
      .digest('hex');
    return signature;
  }

  // Verify HMAC signature
  verifyLogEntry(entry: Record<string, any>, signature: string): boolean {
    const computed = this.signLogEntry(entry);
    return computed === signature;
  }

  // Encrypt sensitive field with AES-256-GCM
  encryptField(plaintext: string, additionalData?: string): EncryptedField {
    const iv = randomBytes(16);
    const cipher = createCipheriv('aes-256-gcm', this.encryptionKey, iv);
    
    if (additionalData) {
      cipher.setAAD(Buffer.from(additionalData));
    }

    const encrypted = Buffer.concat([
      cipher.update(plaintext, 'utf8'),
      cipher.final()
    ]);
    const authTag = cipher.getAuthTag();

    return {
      encrypted_data: encrypted.toString('base64'),
      iv: iv.toString('base64'),
      auth_tag: authTag.toString('base64'),
      algorithm: 'AES-256-GCM',
    };
  }

  // Decrypt field with AES-256-GCM
  decryptField(encrypted: EncryptedField, additionalData?: string): string {
    const iv = Buffer.from(encrypted.iv, 'base64');
    const encryptedData = Buffer.from(encrypted.encrypted_data, 'base64');
    const authTag = Buffer.from(encrypted.auth_tag, 'base64');

    const decipher = createDecipheriv(
      'aes-256-gcm',
      this.encryptionKey,
      iv
    );

    decipher.setAuthTag(authTag);

    if (additionalData) {
      decipher.setAAD(Buffer.from(additionalData));
    }

    let decrypted = decipher.update(encryptedData);
    decrypted = Buffer.concat([decrypted, decipher.final()]);

    return decrypted.toString('utf8');
  }

  // SHA-256 hash (for file integrity)
  hashBuffer(data: Buffer): string {
    return createHash('sha256').update(data).digest('hex');
  }

  // Get HMAC key (for external verification)
  getHmacKey(): Buffer {
    return this.hmacKey;
  }

  // Get encryption key (for external operations)
  getEncryptionKey(): Buffer {
    return this.encryptionKey;
  }
}
