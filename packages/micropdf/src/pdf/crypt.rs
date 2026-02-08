//! PDF encryption and decryption
//!
//! Supports RC4 and AES encryption algorithms with password authentication.

use crate::fitz::error::{Error, Result};
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use md5::{Digest, Md5};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// PDF encryption algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// Keep existing encryption
    Keep,
    /// No encryption
    None,
    /// RC4 with 40-bit key
    Rc4_40,
    /// RC4 with 128-bit key
    Rc4_128,
    /// AES with 128-bit key
    Aes128,
    /// AES with 256-bit key
    Aes256,
    /// Unknown algorithm
    Unknown,
}

impl EncryptionAlgorithm {
    /// Get key length in bytes
    pub fn key_length(&self) -> usize {
        match self {
            Self::Rc4_40 => 5,   // 40 bits = 5 bytes
            Self::Rc4_128 => 16, // 128 bits = 16 bytes
            Self::Aes128 => 16,  // 128 bits = 16 bytes
            Self::Aes256 => 32,  // 256 bits = 32 bytes
            _ => 0,
        }
    }

    /// Check if algorithm uses AES
    pub fn is_aes(&self) -> bool {
        matches!(self, Self::Aes128 | Self::Aes256)
    }

    /// Check if algorithm uses RC4
    pub fn is_rc4(&self) -> bool {
        matches!(self, Self::Rc4_40 | Self::Rc4_128)
    }
}

/// PDF permission flags
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Print = 1 << 2,
    Modify = 1 << 3,
    Copy = 1 << 4,
    Annotate = 1 << 5,
    Form = 1 << 8,
    Accessibility = 1 << 9, // Deprecated in PDF 2.0
    Assemble = 1 << 10,
    PrintHq = 1 << 11,
}

/// PDF encryption context
#[derive(Clone)]
pub struct Crypt {
    /// Encryption algorithm
    algorithm: EncryptionAlgorithm,
    /// Encryption version (1-5)
    version: i32,
    /// Encryption revision (2-6)
    revision: i32,
    /// Key length in bytes
    key_length: usize,
    /// Encryption key
    key: Vec<u8>,
    /// Owner password
    owner_password: Vec<u8>,
    /// User password
    user_password: Vec<u8>,
    /// Permission flags
    permissions: u32,
    /// Encrypt metadata
    encrypt_metadata: bool,
    /// Document ID
    document_id: Vec<u8>,
}

impl Crypt {
    /// Create a new encryption context for decryption
    pub fn new(
        algorithm: EncryptionAlgorithm,
        version: i32,
        revision: i32,
        owner_password: Vec<u8>,
        user_password: Vec<u8>,
        permissions: u32,
        document_id: Vec<u8>,
    ) -> Result<Self> {
        let key_length = algorithm.key_length();
        if key_length == 0 {
            return Err(Error::Generic("Invalid encryption algorithm".to_string()));
        }

        let mut crypt = Self {
            algorithm,
            version,
            revision,
            key_length,
            key: vec![0u8; key_length],
            owner_password,
            user_password,
            permissions,
            encrypt_metadata: true,
            document_id,
        };

        // Compute encryption key
        crypt.compute_encryption_key()?;

        Ok(crypt)
    }

    /// Create encryption context for new document
    pub fn new_encrypt(
        owner_password: &str,
        user_password: &str,
        document_id: Vec<u8>,
        permissions: u32,
        algorithm: EncryptionAlgorithm,
    ) -> Result<Self> {
        let version = match algorithm {
            EncryptionAlgorithm::Rc4_40 => 1,
            EncryptionAlgorithm::Rc4_128 => 2,
            EncryptionAlgorithm::Aes128 => 4,
            EncryptionAlgorithm::Aes256 => 5,
            _ => return Err(Error::Generic("Invalid encryption algorithm".to_string())),
        };

        let revision = match algorithm {
            EncryptionAlgorithm::Rc4_40 => 2,
            EncryptionAlgorithm::Rc4_128 => 3,
            EncryptionAlgorithm::Aes128 => 4,
            EncryptionAlgorithm::Aes256 => 6,
            _ => return Err(Error::Generic("Invalid encryption algorithm".to_string())),
        };

        Self::new(
            algorithm,
            version,
            revision,
            owner_password.as_bytes().to_vec(),
            user_password.as_bytes().to_vec(),
            permissions,
            document_id,
        )
    }

    /// Compute the encryption key
    fn compute_encryption_key(&mut self) -> Result<()> {
        // PDF password padding
        const PADDING: &[u8; 32] = b"\x28\xBF\x4E\x5E\x4E\x75\x8A\x41\x64\x00\x4E\x56\xFF\xFA\x01\x08\x2E\x2E\x00\xB6\xD0\x68\x3E\x80\x2F\x0C\xA9\xFE\x64\x53\x69\x7A";

        // Create MD5 hash
        let mut hasher = Md5::new();

        // Step 1: Pad password
        let mut padded_pwd = self.user_password.clone();
        padded_pwd.resize(32, 0);
        for i in 0..32.min(padded_pwd.len()) {
            if i >= self.user_password.len() {
                padded_pwd[i] = PADDING[i - self.user_password.len()];
            }
        }

        // Step 2: Add password to hash
        hasher.update(&padded_pwd);

        // Step 3: Add owner password
        hasher.update(&self.owner_password);

        // Step 4: Add permissions (little-endian)
        hasher.update(self.permissions.to_le_bytes());

        // Step 5: Add document ID
        hasher.update(&self.document_id);

        // Step 6: If not encrypting metadata (revision 4+), add 0xFFFFFFFF
        if self.revision >= 4 && !self.encrypt_metadata {
            hasher.update([0xFF, 0xFF, 0xFF, 0xFF]);
        }

        let mut key = hasher.finalize().to_vec();

        // Step 7: For revision 3+, hash 50 more times
        if self.revision >= 3 {
            for _ in 0..50 {
                let mut h = Md5::new();
                h.update(&key[..self.key_length]);
                key = h.finalize().to_vec();
            }
        }

        // Truncate to key length
        self.key = key[..self.key_length].to_vec();
        Ok(())
    }

    /// Compute object encryption key
    fn compute_object_key(&self, num: i32, generation: i32) -> Vec<u8> {
        let mut hasher = Md5::new();
        hasher.update(&self.key);
        hasher.update(&num.to_le_bytes()[..3]); // Lower 3 bytes of object number
        hasher.update(&generation.to_le_bytes()[..2]); // Lower 2 bytes of generation

        if self.algorithm.is_aes() {
            hasher.update(b"sAlT"); // AES salt
        }

        let hash = hasher.finalize();
        let key_len = (self.key_length + 5).min(16);
        hash[..key_len].to_vec()
    }

    /// Encrypt data using RC4
    fn encrypt_rc4(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        // Simple RC4 implementation
        let mut s: Vec<u8> = (0..=255).collect();
        let mut j: u8 = 0;

        // Key scheduling
        for i in 0..256 {
            j = j
                .wrapping_add(s[i])
                .wrapping_add(obj_key[i % obj_key.len()]);
            s.swap(i, j as usize);
        }

        // Encryption
        let mut result = Vec::with_capacity(data.len());
        let mut i: u8 = 0;
        let mut j: u8 = 0;

        for &byte in data {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[i as usize]);
            s.swap(i as usize, j as usize);
            let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
            result.push(byte ^ k);
        }

        Ok(result)
    }

    /// Decrypt data using RC4 (same as encrypt for RC4)
    fn decrypt_rc4(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        self.encrypt_rc4(data, obj_key)
    }

    /// Encrypt data using AES-128
    fn encrypt_aes128(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        // Generate random IV (16 bytes for AES)
        let iv = [0u8; 16]; // In production, use random IV

        // Pad data to block size (16 bytes)
        let mut padded = data.to_vec();
        let padding_len = 16 - (data.len() % 16);
        padded.extend(vec![padding_len as u8; padding_len]);

        // Encrypt
        let cipher = Aes128CbcEnc::new_from_slices(obj_key, &iv)
            .map_err(|e| Error::Generic(format!("AES key/IV error: {:?}", e)))?;

        let mut result = vec![0u8; padded.len()];
        cipher
            .encrypt_padded_b2b_mut::<aes::cipher::block_padding::NoPadding>(&padded, &mut result)
            .map_err(|e| Error::Generic(format!("AES encryption error: {:?}", e)))?;

        // Prepend IV
        let mut output = iv.to_vec();
        output.extend(result);
        Ok(output)
    }

    /// Decrypt data using AES-128
    fn decrypt_aes128(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 16 {
            return Err(Error::Generic("Invalid AES encrypted data".to_string()));
        }

        // Extract IV (first 16 bytes)
        let iv = &data[..16];
        let encrypted = &data[16..];

        // Decrypt
        let cipher = Aes128CbcDec::new_from_slices(obj_key, iv)
            .map_err(|e| Error::Generic(format!("AES key/IV error: {:?}", e)))?;

        let mut result = vec![0u8; encrypted.len()];
        cipher
            .decrypt_padded_b2b_mut::<aes::cipher::block_padding::Pkcs7>(encrypted, &mut result)
            .map_err(|e| Error::Generic(format!("AES decryption error: {:?}", e)))?;

        Ok(result)
    }

    /// Encrypt data using AES-256
    fn encrypt_aes256(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        let iv = [0u8; 16]; // In production, use random IV

        let mut padded = data.to_vec();
        let padding_len = 16 - (data.len() % 16);
        padded.extend(vec![padding_len as u8; padding_len]);

        let cipher = Aes256CbcEnc::new_from_slices(obj_key, &iv)
            .map_err(|e| Error::Generic(format!("AES key/IV error: {:?}", e)))?;

        let mut result = vec![0u8; padded.len()];
        cipher
            .encrypt_padded_b2b_mut::<aes::cipher::block_padding::NoPadding>(&padded, &mut result)
            .map_err(|e| Error::Generic(format!("AES encryption error: {:?}", e)))?;

        let mut output = iv.to_vec();
        output.extend(result);
        Ok(output)
    }

    /// Decrypt data using AES-256
    fn decrypt_aes256(&self, data: &[u8], obj_key: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 16 {
            return Err(Error::Generic("Invalid AES encrypted data".to_string()));
        }

        let iv = &data[..16];
        let encrypted = &data[16..];

        let cipher = Aes256CbcDec::new_from_slices(obj_key, iv)
            .map_err(|e| Error::Generic(format!("AES key/IV error: {:?}", e)))?;

        let mut result = vec![0u8; encrypted.len()];
        cipher
            .decrypt_padded_b2b_mut::<aes::cipher::block_padding::Pkcs7>(encrypted, &mut result)
            .map_err(|e| Error::Generic(format!("AES decryption error: {:?}", e)))?;

        Ok(result)
    }

    /// Encrypt data for a specific object
    pub fn encrypt_data(&self, data: &[u8], obj_num: i32, obj_generation: i32) -> Result<Vec<u8>> {
        if self.algorithm == EncryptionAlgorithm::None {
            return Ok(data.to_vec());
        }

        let obj_key = self.compute_object_key(obj_num, obj_generation);

        match self.algorithm {
            EncryptionAlgorithm::Rc4_40 | EncryptionAlgorithm::Rc4_128 => {
                self.encrypt_rc4(data, &obj_key)
            }
            EncryptionAlgorithm::Aes128 => self.encrypt_aes128(data, &obj_key),
            EncryptionAlgorithm::Aes256 => self.encrypt_aes256(data, &obj_key),
            _ => Err(Error::Generic(
                "Unsupported encryption algorithm".to_string(),
            )),
        }
    }

    /// Decrypt data for a specific object
    pub fn decrypt_data(&self, data: &[u8], obj_num: i32, obj_generation: i32) -> Result<Vec<u8>> {
        if self.algorithm == EncryptionAlgorithm::None {
            return Ok(data.to_vec());
        }

        let obj_key = self.compute_object_key(obj_num, obj_generation);

        match self.algorithm {
            EncryptionAlgorithm::Rc4_40 | EncryptionAlgorithm::Rc4_128 => {
                self.decrypt_rc4(data, &obj_key)
            }
            EncryptionAlgorithm::Aes128 => self.decrypt_aes128(data, &obj_key),
            EncryptionAlgorithm::Aes256 => self.decrypt_aes256(data, &obj_key),
            _ => Err(Error::Generic(
                "Unsupported encryption algorithm".to_string(),
            )),
        }
    }

    /// Get encryption version
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Get encryption revision
    pub fn revision(&self) -> i32 {
        self.revision
    }

    /// Get algorithm
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
    }

    /// Get key length in bytes
    pub fn key_length(&self) -> usize {
        self.key_length
    }

    /// Get permissions
    pub fn permissions(&self) -> u32 {
        self.permissions
    }

    /// Check if metadata is encrypted
    pub fn encrypt_metadata(&self) -> bool {
        self.encrypt_metadata
    }

    /// Check if a permission is granted
    pub fn has_permission(&self, perm: Permission) -> bool {
        (self.permissions & (perm as u32)) != 0
    }

    /// Get encryption method name
    pub fn method_name(&self) -> &str {
        match self.algorithm {
            EncryptionAlgorithm::None => "None",
            EncryptionAlgorithm::Rc4_40 => "RC4 (40-bit)",
            EncryptionAlgorithm::Rc4_128 => "RC4 (128-bit)",
            EncryptionAlgorithm::Aes128 => "AES (128-bit)",
            EncryptionAlgorithm::Aes256 => "AES (256-bit)",
            _ => "Unknown",
        }
    }
}

impl std::fmt::Debug for Crypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crypt")
            .field("algorithm", &self.algorithm)
            .field("version", &self.version)
            .field("revision", &self.revision)
            .field("key_length", &self.key_length)
            .field("permissions", &format!("0x{:08X}", self.permissions))
            .field("encrypt_metadata", &self.encrypt_metadata)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_algorithm_key_length() {
        assert_eq!(EncryptionAlgorithm::Rc4_40.key_length(), 5);
        assert_eq!(EncryptionAlgorithm::Rc4_128.key_length(), 16);
        assert_eq!(EncryptionAlgorithm::Aes128.key_length(), 16);
        assert_eq!(EncryptionAlgorithm::Aes256.key_length(), 32);
        assert_eq!(EncryptionAlgorithm::None.key_length(), 0);
    }

    #[test]
    fn test_encryption_algorithm_types() {
        assert!(EncryptionAlgorithm::Rc4_40.is_rc4());
        assert!(EncryptionAlgorithm::Rc4_128.is_rc4());
        assert!(!EncryptionAlgorithm::Aes128.is_rc4());

        assert!(EncryptionAlgorithm::Aes128.is_aes());
        assert!(EncryptionAlgorithm::Aes256.is_aes());
        assert!(!EncryptionAlgorithm::Rc4_40.is_aes());
    }

    #[test]
    fn test_permission_flags() {
        assert_eq!(Permission::Print as u32, 1 << 2);
        assert_eq!(Permission::Modify as u32, 1 << 3);
        assert_eq!(Permission::Copy as u32, 1 << 4);
        assert_eq!(Permission::PrintHq as u32, 1 << 11);
    }

    #[test]
    fn test_crypt_new() {
        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let crypt = Crypt::new(
            EncryptionAlgorithm::Rc4_128,
            2,
            3,
            b"owner".to_vec(),
            b"user".to_vec(),
            0xFFFFF0C0,
            doc_id,
        );

        assert!(crypt.is_ok());
        let crypt = crypt.unwrap();
        assert_eq!(crypt.version(), 2);
        assert_eq!(crypt.revision(), 3);
        assert_eq!(crypt.key_length(), 16);
        assert_eq!(crypt.algorithm(), EncryptionAlgorithm::Rc4_128);
    }

    #[test]
    fn test_crypt_new_encrypt() {
        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let crypt = Crypt::new_encrypt(
            "owner_password",
            "user_password",
            doc_id,
            0xFFFFF0C0,
            EncryptionAlgorithm::Aes128,
        );

        assert!(crypt.is_ok());
        let crypt = crypt.unwrap();
        assert_eq!(crypt.version(), 4);
        assert_eq!(crypt.revision(), 4);
        assert_eq!(crypt.algorithm(), EncryptionAlgorithm::Aes128);
    }

    #[test]
    fn test_rc4_encrypt_decrypt() {
        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let crypt = Crypt::new(
            EncryptionAlgorithm::Rc4_128,
            2,
            3,
            b"owner".to_vec(),
            b"user".to_vec(),
            0xFFFFF0C0,
            doc_id,
        )
        .unwrap();

        let original = b"Hello, World! This is a test.";
        let encrypted = crypt.encrypt_data(original, 1, 0).unwrap();
        assert_ne!(encrypted, original);

        let decrypted = crypt.decrypt_data(&encrypted, 1, 0).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_permissions() {
        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let perms = Permission::Print as u32 | Permission::Copy as u32;
        let crypt = Crypt::new(
            EncryptionAlgorithm::Aes128,
            4,
            4,
            b"owner".to_vec(),
            b"user".to_vec(),
            perms,
            doc_id,
        )
        .unwrap();

        assert!(crypt.has_permission(Permission::Print));
        assert!(crypt.has_permission(Permission::Copy));
        assert!(!crypt.has_permission(Permission::Modify));
        assert!(!crypt.has_permission(Permission::Annotate));
    }

    #[test]
    fn test_method_names() {
        assert!(!EncryptionAlgorithm::None.is_rc4());

        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let crypt = Crypt::new(
            EncryptionAlgorithm::Rc4_128,
            2,
            3,
            b"owner".to_vec(),
            b"user".to_vec(),
            0xFFFFF0C0,
            doc_id,
        )
        .unwrap();

        assert_eq!(crypt.method_name(), "RC4 (128-bit)");
    }

    #[test]
    fn test_no_encryption() {
        let doc_id = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let original = b"Test data";

        // Create crypt with RC4 first, then test None algorithm
        let crypt = Crypt::new(
            EncryptionAlgorithm::Rc4_128,
            2,
            3,
            b"owner".to_vec(),
            b"user".to_vec(),
            0xFFFFF0C0,
            doc_id.clone(),
        )
        .unwrap();

        // Manually set algorithm to None for testing
        let mut no_crypt = crypt.clone();
        no_crypt.algorithm = EncryptionAlgorithm::None;

        let encrypted = no_crypt.encrypt_data(original, 1, 0).unwrap();
        assert_eq!(encrypted, original); // Should be unchanged

        let decrypted = no_crypt.decrypt_data(&encrypted, 1, 0).unwrap();
        assert_eq!(decrypted, original);
    }
}
