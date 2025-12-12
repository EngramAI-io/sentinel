import { AUDIT_CONFIG } from './config';
import { CryptoEngine } from './crypto';
import * as fs from 'fs/promises';
import * as path from 'path';

export class AppendOnlyStore {
  private logDir: string;
  private crypto: CryptoEngine;

  constructor() {
    this.logDir = AUDIT_CONFIG.logging.dir;
    this.crypto = new CryptoEngine();
    if (AUDIT_CONFIG.features.AUDIT_STORAGE) {
      this.ensureDir();
    }
  }

  private async ensureDir(): Promise<void> {
    try {
      await fs.mkdir(this.logDir, { recursive: true });
    } catch (err) {
      console.error('[AUDIT] Failed to create log dir:', err);
    }
  }

  async append(entry: Record<string, any>): Promise<void> {
    if (!AUDIT_CONFIG.features.AUDIT_STORAGE) return;

    const date = new Date().toISOString().split('T')[0];
    const filePath = path.join(this.logDir, `audit-${date}.jsonl`);

    try {
      // Append-only: always append, never overwrite
      const logLine = JSON.stringify(entry) + '\n';
      await fs.appendFile(filePath, logLine, { flag: 'a' });

      // If WORM enabled, make file read-only after append
      if (AUDIT_CONFIG.storage.writeOnce && process.platform !== 'win32') {
        // After file reaches certain size, make immutable
        const stats = await fs.stat(filePath);
        if (stats.size > 10_000_000) { // 10MB
          await fs.chmod(filePath, 0o444); // Read-only
        }
      }
    } catch (err) {
      console.error('[AUDIT] Failed to write log:', err);
    }
  }

  async verifyIntegrity(filePath: string): Promise<boolean> {
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      const entries = content.split('\n').filter(Boolean);

      for (const line of entries) {
        const entry = JSON.parse(line);
        if (AUDIT_CONFIG.features.TAMPER_EVIDENT && entry.hmac_signature) {
          const signature = entry.hmac_signature;
          delete entry.hmac_signature;
          const isValid = this.crypto.verifyLogEntry(entry, signature);
          if (!isValid) {
            console.warn('[AUDIT] TAMPER DETECTED:', filePath, line.substring(0, 50));
            return false;
          }
        }
      }
      return true;
    } catch (err) {
      console.error('[AUDIT] Failed to verify integrity:', err);
      return false;
    }
  }

  async query(filters?: Record<string, any>): Promise<any[]> {
    if (!AUDIT_CONFIG.features.AUDIT_STORAGE) return [];

    const results: any[] = [];

    try {
      const files = await fs.readdir(this.logDir);
      for (const file of files) {
        if (!file.startsWith('audit-')) continue;

        const filePath = path.join(this.logDir, file);
        const content = await fs.readFile(filePath, 'utf-8');
        const entries = content.split('\n').filter(Boolean);

        for (const line of entries) {
          const entry = JSON.parse(line);
          if (this._matchesFilters(entry, filters)) {
            results.push(entry);
          }
        }
      }
    } catch (err) {
      console.error('[AUDIT] Failed to query logs:', err);
    }

    return results;
  }

  private _matchesFilters(entry: any, filters?: Record<string, any>): boolean {
    if (!filters) return true;
    if (filters.event_type && entry.event_type !== filters.event_type) return false;
    if (filters.level && entry.level !== filters.level) return false;
    if (filters.session_id && entry.session_id !== filters.session_id) return false;
    if (filters.user_id && entry.user_id !== filters.user_id) return false;
    return true;
  }

  async getLogDir(): Promise<string> {
    return this.logDir;
  }
}
