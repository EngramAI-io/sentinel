"""
Cryptographic operations for audit logs (Python implementation)
Provides HMAC-SHA256 signing and AES-256-GCM encryption
"""

import hashlib
import hmac
import os
import base64
from typing import Dict, Any, Optional
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.backends import default_backend


class CryptoEngine:
    """Cryptographic engine for audit log integrity and encryption"""
    
    def __init__(self, hmac_key: Optional[bytes] = None, encryption_key: Optional[bytes] = None):
        """
        Initialize crypto engine with keys from environment or generate new ones
        
        Args:
            hmac_key: HMAC key (32 bytes) or None to use env/generate
            encryption_key: AES-256 key (32 bytes) or None to use env/generate
        """
        # HMAC key
        hmac_key_env = os.getenv('AUDIT_HMAC_KEY', '')
        if hmac_key:
            self.hmac_key = hmac_key
        elif hmac_key_env:
            self.hmac_key = bytes.fromhex(hmac_key_env)
        else:
            self.hmac_key = os.urandom(32)
            print('[CRYPTO] Generated new HMAC key (not for production)')
        
        # Encryption key
        enc_key_env = os.getenv('AUDIT_ENCRYPTION_KEY', '')
        if encryption_key:
            self.encryption_key = encryption_key
        elif enc_key_env:
            self.encryption_key = bytes.fromhex(enc_key_env)
        else:
            self.encryption_key = os.urandom(32)
            print('[CRYPTO] Generated new encryption key (not for production)')
    
    def sign_log_entry(self, entry: Dict[str, Any]) -> str:
        """
        Generate HMAC-SHA256 signature for log entry
        
        Args:
            entry: Log entry dictionary
            
        Returns:
            Hex-encoded HMAC signature
        """
        import json
        entry_str = json.dumps(entry, sort_keys=True)
        signature = hmac.new(
            self.hmac_key,
            entry_str.encode('utf-8'),
            hashlib.sha256
        ).hexdigest()
        return signature
    
    def verify_log_entry(self, entry: Dict[str, Any], signature: str) -> bool:
        """
        Verify HMAC signature of log entry
        
        Args:
            entry: Log entry dictionary
            signature: Expected signature (hex)
            
        Returns:
            True if signature is valid
        """
        computed = self.sign_log_entry(entry)
        return hmac.compare_digest(computed, signature)
    
    def encrypt_field(self, plaintext: str, additional_data: Optional[str] = None) -> Dict[str, str]:
        """
        Encrypt sensitive field with AES-256-GCM
        
        Args:
            plaintext: Text to encrypt
            additional_data: Optional authenticated additional data
            
        Returns:
            Dictionary with encrypted_data, iv, auth_tag, algorithm
        """
        aesgcm = AESGCM(self.encryption_key)
        nonce = os.urandom(12)  # 96-bit nonce for GCM
        
        aad = additional_data.encode('utf-8') if additional_data else b''
        ciphertext = aesgcm.encrypt(nonce, plaintext.encode('utf-8'), aad)
        
        # Extract auth tag (last 16 bytes) and ciphertext
        auth_tag = ciphertext[-16:]
        encrypted_data = ciphertext[:-16]
        
        return {
            'encrypted_data': base64.b64encode(encrypted_data).decode('utf-8'),
            'iv': base64.b64encode(nonce).decode('utf-8'),
            'auth_tag': base64.b64encode(auth_tag).decode('utf-8'),
            'algorithm': 'AES-256-GCM',
        }
    
    def decrypt_field(self, encrypted: Dict[str, str], additional_data: Optional[str] = None) -> str:
        """
        Decrypt field with AES-256-GCM
        
        Args:
            encrypted: Dictionary with encrypted_data, iv, auth_tag
            additional_data: Optional authenticated additional data
            
        Returns:
            Decrypted plaintext
        """
        aesgcm = AESGCM(self.encryption_key)
        nonce = base64.b64decode(encrypted['iv'])
        encrypted_data = base64.b64decode(encrypted['encrypted_data'])
        auth_tag = base64.b64decode(encrypted['auth_tag'])
        
        # Combine ciphertext and auth tag
        ciphertext = encrypted_data + auth_tag
        
        aad = additional_data.encode('utf-8') if additional_data else b''
        plaintext = aesgcm.decrypt(nonce, ciphertext, aad)
        
        return plaintext.decode('utf-8')
    
    def hash_buffer(self, data: bytes) -> str:
        """
        SHA-256 hash of buffer (for file integrity)
        
        Args:
            data: Data to hash
            
        Returns:
            Hex-encoded SHA-256 hash
        """
        return hashlib.sha256(data).hexdigest()
