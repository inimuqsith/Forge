use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey,
};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

/// Keypair untuk menandatangani paket biner Forge
#[derive(Clone)]
pub struct SigningKeyPair {
    signing_key: SigningKey,
}

impl SigningKeyPair {
    /// Buat pasangan kunci baru menggunakan entropy sistem (OsRng)
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Muat signing key dari 32 byte secret
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        Self { signing_key }
    }

    /// Muat signing key dari string hex 64 karakter
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let clean_hex = hex_str.trim();
        let bytes = hex::decode(clean_hex).context("Gagal mem-parsing string hex signing key")?;
        if bytes.len() != 32 {
            return Err(anyhow!(
                "Panjang secret key Ed25519 harus 32 byte (64 karakter hex), didapat: {} byte",
                bytes.len()
            ));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Ok(Self::from_bytes(&key_bytes))
    }

    /// Muat signing key dari berkas
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Gagal membaca signing key dari {:?}", path))?;
        Self::from_hex(&content)
    }

    /// Dapatkan verifying key (kunci publik)
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Dapatkan representasi hex dari secret key
    pub fn to_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    /// Dapatkan representasi hex dari public key
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key().to_bytes())
    }

    /// Simpan pasangan kunci ke berkas (secret key dan opsional public key)
    pub fn save_to_files(&self, secret_path: &Path, pub_path: Option<&Path>) -> Result<()> {
        if let Some(parent) = secret_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(secret_path, format!("{}\n", self.to_hex()))
            .with_context(|| format!("Gagal menulis secret key ke {:?}", secret_path))?;

        if let Some(pub_p) = pub_path {
            if let Some(parent) = pub_p.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(pub_p, format!("{}\n", self.public_key_hex()))
                .with_context(|| format!("Gagal menulis public key ke {:?}", pub_p))?;
        }

        Ok(())
    }

    /// Tandatangani slice data byte
    pub fn sign_bytes(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }

    /// Tandatangani berkas langsung dari disk
    pub fn sign_file(&self, path: &Path) -> Result<Signature> {
        let data = fs::read(path)
            .with_context(|| format!("Gagal membaca berkas untuk signing: {:?}", path))?;
        Ok(self.sign_bytes(&data))
    }

    /// Tandatangani berkas dan simpan hasilnya ke file `.sig` (format hex)
    pub fn sign_file_to_sig_file(&self, file_path: &Path, sig_path: &Path) -> Result<()> {
        let sig = self.sign_file(file_path)?;
        let sig_hex = hex::encode(sig.to_bytes());
        fs::write(sig_path, format!("{}\n", sig_hex))
            .with_context(|| format!("Gagal menulis signature ke {:?}", sig_path))?;
        Ok(())
    }
}

/// Verifier tanda tangan kriptografis paket biner
#[derive(Clone, Debug)]
pub struct PackageVerifier {
    verifying_key: VerifyingKey,
}

impl PackageVerifier {
    /// Inisialisasi verifier dari VerifyingKey
    pub fn from_verifying_key(verifying_key: VerifyingKey) -> Self {
        Self { verifying_key }
    }

    /// Inisialisasi verifier dari 32 byte array
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let verifying_key = VerifyingKey::from_bytes(bytes)
            .map_err(|e| anyhow!("Kunci publik Ed25519 tidak valid: {}", e))?;
        Ok(Self { verifying_key })
    }

    /// Inisialisasi verifier dari string hex 64 karakter
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let clean_hex = hex_str.trim();
        let bytes = hex::decode(clean_hex).context("Gagal mem-parsing string hex public key")?;
        if bytes.len() != 32 {
            return Err(anyhow!(
                "Panjang public key Ed25519 harus 32 byte (64 karakter hex), didapat: {} byte",
                bytes.len()
            ));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Self::from_bytes(&key_bytes)
    }

    /// Muat public key dari berkas disk
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Gagal membaca public key dari {:?}", path))?;
        Self::from_hex(&content)
    }

    /// Verifikasi tanda tangan terhadap data byte
    pub fn verify_bytes(&self, data: &[u8], signature: &Signature) -> Result<()> {
        self.verifying_key
            .verify(data, signature)
            .map_err(|e| anyhow!("Verifikasi tanda tangan digital Ed25519 GAGAL: {}", e))
    }

    /// Verifikasi tanda tangan terhadap isi berkas
    pub fn verify_file(&self, path: &Path, signature: &Signature) -> Result<()> {
        let data = fs::read(path)
            .with_context(|| format!("Gagal membaca berkas untuk verifikasi signature: {:?}", path))?;
        self.verify_bytes(&data, signature)
    }

    /// Verifikasi berkas terhadap file signature `.sig` (format hex)
    pub fn verify_file_sig_file(&self, file_path: &Path, sig_path: &Path) -> Result<()> {
        let sig_content = fs::read_to_string(sig_path)
            .with_context(|| format!("Gagal membaca berkas signature {:?}", sig_path))?;
        let sig_bytes = hex::decode(sig_content.trim())
            .context("Gagal decode isi signature hex")?;
        if sig_bytes.len() != 64 {
            return Err(anyhow!(
                "Panjang signature Ed25519 harus 64 byte (128 karakter hex), didapat: {} byte",
                sig_bytes.len()
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);
        self.verify_file(file_path, &signature)
    }

    /// Dapatkan default system verifier jika ada di `/etc/forge/keys/kura.pub` atau env `FORGE_PUBKEY`
    pub fn default_system_verifier() -> Option<Self> {
        if let Ok(env_key) = std::env::var("FORGE_PUBKEY") {
            if let Ok(v) = Self::from_hex(&env_key) {
                return Some(v);
            }
        }

        let paths = [
            Path::new("/etc/forge/keys/kura.pub"),
            Path::new("./config/keys/kura.pub"),
        ];

        for p in paths {
            if p.exists() {
                if let Ok(v) = Self::load_from_file(p) {
                    return Some(v);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_ed25519_keygen_sign_verify_cycle() -> Result<()> {
        let keypair = SigningKeyPair::generate();
        let pubkey_hex = keypair.public_key_hex();
        assert_eq!(pubkey_hex.len(), 64);

        let data = b"Forge Package Manager binary payload test";
        let signature = keypair.sign_bytes(data);

        let verifier = PackageVerifier::from_hex(&pubkey_hex)?;
        assert!(verifier.verify_bytes(data, &signature).is_ok());

        // Verifikasi tamper rejection
        let corrupted_data = b"Forge Package Manager binary payload test - TAMPERED";
        assert!(verifier.verify_bytes(corrupted_data, &signature).is_err());

        Ok(())
    }

    #[test]
    fn test_ed25519_file_signature_roundtrip() -> Result<()> {
        let temp = tempdir()?;
        let pkg_file = temp.path().join("gcc-15.3.0.forge.tar.zst");
        let sig_file = temp.path().join("gcc-15.3.0.forge.tar.zst.sig");
        let priv_key_file = temp.path().join("kura.priv");
        let pub_key_file = temp.path().join("kura.pub");

        fs::write(&pkg_file, b"Mock binary tarball contents")?;

        let keypair = SigningKeyPair::generate();
        keypair.save_to_files(&priv_key_file, Some(&pub_key_file))?;

        // Muat ulang dari disk
        let loaded_keypair = SigningKeyPair::load_from_file(&priv_key_file)?;
        loaded_keypair.sign_file_to_sig_file(&pkg_file, &sig_file)?;

        let verifier = PackageVerifier::load_from_file(&pub_key_file)?;
        assert!(verifier.verify_file_sig_file(&pkg_file, &sig_file).is_ok());

        // Modifikasi isi berkas untuk memastikan signature gagal
        fs::write(&pkg_file, b"Hacked mock binary tarball contents")?;
        assert!(verifier.verify_file_sig_file(&pkg_file, &sig_file).is_err());

        Ok(())
    }
}
