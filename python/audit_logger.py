"""
Tamper-evident structured logging with cryptographic integrity
"""

import json
import os
from datetime import datetime
from typing import Dict, Any, Optional
from audit_crypto import CryptoEngine


class TamperEvidenceLogger:
    """Structured logger with HMAC signatures and field encryption"""
    
    def __init__(self):
        self.crypto = CryptoEngine()
        self.session_id = ''
        self.agent_id = 'unknown'
        self.user_id = os.getenv('SENTINEL_USER_ID', 'anonymous')
        
        # Feature flags
        self.structured_logging = os.getenv('SENTINEL_FEATURE_LOGGING', 'false').lower() == 'true'
        self.field_encryption = os.getenv('SENTINEL_FEATURE_ENCRYPTION', 'false').lower() == 'true'
        self.tamper_evident = os.getenv('SENTINEL_FEATURE_TAMPER_EVIDENT', 'false').lower() == 'true'
        
        # Fields to encrypt
        encrypt_fields_str = os.getenv('AUDIT_ENCRYPT_FIELDS', 'password,api_key,token,db_url')
        self.encrypt_fields = [f.strip() for f in encrypt_fields_str.split(',')]
    
    def set_session_id(self, session_id: str, agent_id: str = 'unknown'):
        """Set session and agent identifiers"""
        self.session_id = session_id
        self.agent_id = agent_id
    
    def info(self, event_type: str, message: str, payload: Optional[Dict[str, Any]] = None):
        """Log info-level event"""
        self._log('INFO', event_type, message, payload)
    
    def error(self, event_type: str, message: str, payload: Optional[Dict[str, Any]] = None):
        """Log error-level event"""
        self._log('ERROR', event_type, message, payload)
    
    def warn(self, event_type: str, message: str, payload: Optional[Dict[str, Any]] = None):
        """Log warning-level event"""
        self._log('WARN', event_type, message, payload)
    
    def critical(self, event_type: str, message: str, payload: Optional[Dict[str, Any]] = None):
        """Log critical-level event"""
        self._log('CRITICAL', event_type, message, payload)
    
    def _log(self, level: str, event_type: str, message: str, payload: Optional[Dict[str, Any]]):
        """Internal logging method"""
        if not self.structured_logging:
            return
        
        # Build base entry
        entry: Dict[str, Any] = {
            'timestamp': datetime.utcnow().isoformat() + 'Z',
            'level': level,
            'event_type': event_type,
            'session_id': self.session_id or 'unknown',
            'agent_id': self.agent_id,
            'user_id': self.user_id,
            'correlation_id': self._generate_correlation_id(),
            'schema_version': '1.0',
            'context_snapshot': {
                'environment': os.getenv('NODE_ENV', 'development'),
                'timestamp_ms': int(datetime.utcnow().timestamp() * 1000),
            },
            'classification': 'CONFIDENTIAL',
            'source_file': self._get_source_file(),
            'tags': [event_type.split('_')[0]],
            'payload': payload or {},
            'encrypted_fields': [],
        }
        
        # Encrypt sensitive fields
        if self.field_encryption and payload:
            entry = self._encrypt_sensitive_fields(entry)
        
        # Generate HMAC signature
        if self.tamper_evident:
            entry_for_signature = entry.copy()
            entry_for_signature.pop('hmac_signature', None)
            entry['hmac_signature'] = self.crypto.sign_log_entry(entry_for_signature)
        else:
            entry['hmac_signature'] = ''
        
        # Output JSON
        print(json.dumps(entry))
    
    def _encrypt_sensitive_fields(self, entry: Dict[str, Any]) -> Dict[str, Any]:
        """Encrypt sensitive fields in payload"""
        if 'payload' not in entry:
            return entry
        
        for field_name in self.encrypt_fields:
            if field_name in entry['payload']:
                plaintext = str(entry['payload'][field_name])
                entry['payload'][field_name] = self.crypto.encrypt_field(
                    plaintext,
                    entry.get('session_id', '')
                )
                entry['encrypted_fields'].append(field_name)
        
        return entry
    
    def _generate_correlation_id(self) -> str:
        """Generate correlation ID"""
        import random
        import string
        return 'req-' + ''.join(random.choices(string.ascii_lowercase + string.digits, k=9))
    
    def _get_source_file(self) -> str:
        """Get source file from stack trace"""
        import traceback
        try:
            stack = traceback.extract_stack()
            if len(stack) > 2:
                frame = stack[-3]
                return f"{frame.filename}:{frame.lineno}"
        except:
            pass
        return 'unknown.py:0'
