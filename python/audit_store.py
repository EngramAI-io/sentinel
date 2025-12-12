"""
Append-only immutable storage for audit logs
"""

import os
import json
from datetime import datetime
from pathlib import Path
from typing import Dict, Any, List, Optional
from audit_crypto import CryptoEngine


class AppendOnlyStore:
    """Append-only WORM storage with integrity verification"""
    
    def __init__(self):
        self.log_dir = os.getenv('AUDIT_LOG_DIR', './logs/audit')
        self.crypto = CryptoEngine()
        self.write_once = os.getenv('AUDIT_WRITE_ONCE', 'false').lower() == 'true'
        self.tamper_evident = os.getenv('SENTINEL_FEATURE_TAMPER_EVIDENT', 'false').lower() == 'true'
        
        # Ensure directory exists
        Path(self.log_dir).mkdir(parents=True, exist_ok=True)
    
    def append(self, entry: Dict[str, any]) -> None:
        """
        Append log entry to append-only file
        
        Args:
            entry: Log entry dictionary
        """
        date_str = datetime.utcnow().strftime('%Y-%m-%d')
        file_path = Path(self.log_dir) / f'audit-{date_str}.jsonl'
        
        try:
            # Append-only: always append, never overwrite
            log_line = json.dumps(entry) + '\n'
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(log_line)
            
            # If WORM enabled, make file read-only after certain size
            if self.write_once and os.name != 'nt':  # Not Windows
                file_size = file_path.stat().st_size
                if file_size > 10_000_000:  # 10MB
                    os.chmod(file_path, 0o444)  # Read-only
        except Exception as e:
            print(f'[AUDIT] Failed to write log: {e}')
    
    def verify_integrity(self, file_path: str) -> bool:
        """
        Verify integrity of log file
        
        Args:
            file_path: Path to log file
            
        Returns:
            True if all entries are valid
        """
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue
                    
                    entry = json.loads(line)
                    if self.tamper_evident and entry.get('hmac_signature'):
                        signature = entry.pop('hmac_signature')
                        is_valid = self.crypto.verify_log_entry(entry, signature)
                        if not is_valid:
                            print(f'[AUDIT] TAMPER DETECTED: {file_path} line {line_num}')
                            return False
                        entry['hmac_signature'] = signature  # Restore
            return True
        except Exception as e:
            print(f'[AUDIT] Failed to verify integrity: {e}')
            return False
    
    def query(self, filters: Optional[Dict[str, Any]] = None) -> List[Dict[str, Any]]:
        """
        Query log entries with filters
        
        Args:
            filters: Dictionary of filter criteria
            
        Returns:
            List of matching log entries
        """
        results = []
        log_dir = Path(self.log_dir)
        
        try:
            for file_path in log_dir.glob('audit-*.jsonl'):
                with open(file_path, 'r', encoding='utf-8') as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        
                        entry = json.loads(line)
                        if self._matches_filters(entry, filters):
                            results.append(entry)
        except Exception as e:
            print(f'[AUDIT] Failed to query logs: {e}')
        
        return results
    
    def _matches_filters(self, entry: Dict[str, Any], filters: Optional[Dict[str, Any]]) -> bool:
        """Check if entry matches filters"""
        if not filters:
            return True
        
        if 'event_type' in filters and entry.get('event_type') != filters['event_type']:
            return False
        if 'level' in filters and entry.get('level') != filters['level']:
            return False
        if 'session_id' in filters and entry.get('session_id') != filters['session_id']:
            return False
        if 'user_id' in filters and entry.get('user_id') != filters['user_id']:
            return False
        
        return True
