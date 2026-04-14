use core::fmt;

use fd_ed25519::Pubkey;
use fd_txn_sys as sys;

use crate::TransactionVersion;

const ACCT_MAX: usize = sys::FD_TXN_ACCT_ADDR_MAX as usize;
const INSTR_MAX: usize = sys::FD_TXN_INSTR_MAX as usize;
const MTU: usize = sys::FD_TXN_MTU as usize;
const SIG_SZ: usize = sys::FD_TXN_SIGNATURE_SZ as usize;

const ACCT_F_SIGNER: u8 = 0x01;
const ACCT_F_WRITABLE: u8 = 0x02;

const ZERO_KEY: Pubkey = Pubkey::from_bytes(&[0; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    #[inline]
    pub fn new(pubkey: Pubkey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }

    #[inline]
    pub fn readonly(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: false,
            is_writable: false,
        }
    }

    #[inline]
    pub fn writable(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: false,
            is_writable: true,
        }
    }

    #[inline]
    pub fn signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: true,
            is_writable: false,
        }
    }

    #[inline]
    pub fn writable_signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: true,
            is_writable: true,
        }
    }

    #[inline]
    pub fn readonly_signer(pubkey: Pubkey) -> Self {
        Self {
            pubkey,
            is_signer: true,
            is_writable: false,
        }
    }
}

pub struct InstructionBuf<'a> {
    pub program_id: Pubkey,
    pub accounts: &'a [AccountMeta],
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    NoFeePayer,
    NoBlockhash,
    NoInstructions,
    TooManyAccounts,
    TooManyInstructions,
    TooManySigners,
    DataTooLarge,
    TransactionTooLarge,
    ValidationFailed,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::NoFeePayer => write!(f, "No fee payer set"),
            BuildError::NoBlockhash => write!(f, "No recent blockhash set"),
            BuildError::NoInstructions => write!(f, "No instructions added"),
            BuildError::TooManyAccounts => write!(f, "Too many accounts (max {})", ACCT_MAX),
            BuildError::TooManyInstructions => {
                write!(f, "Too many instructions (max {})", INSTR_MAX)
            }
            BuildError::TooManySigners => {
                write!(f, "Too many signers (max {})", sys::FD_TXN_SIG_MAX)
            }
            BuildError::DataTooLarge => write!(f, "Instruction data exceeds capacity"),
            BuildError::TransactionTooLarge => {
                write!(f, "Serialized transaction exceeds MTU ({} bytes)", MTU)
            }
            BuildError::ValidationFailed => {
                write!(f, "Transaction failed round-trip parse validation")
            }
        }
    }
}

impl core::error::Error for BuildError {}

#[derive(Clone, Copy)]
struct InstrEntry {
    program_idx: u8,
    acct_off: u16,
    acct_cnt: u16,
    data_off: u16,
    data_len: u16,
}

const ZERO_INSTR: InstrEntry = InstrEntry {
    program_idx: 0,
    acct_off: 0,
    acct_cnt: 0,
    data_off: 0,
    data_len: 0,
};

pub struct TransactionBuilder {
    acct_keys: [Pubkey; ACCT_MAX],
    acct_flags: [u8; ACCT_MAX],
    acct_count: usize,

    instrs: [InstrEntry; INSTR_MAX],
    instr_count: usize,

    instr_acct_buf: [u8; MTU],
    instr_acct_len: usize,

    instr_data_buf: [u8; MTU],
    instr_data_len: usize,

    fee_payer: Option<Pubkey>,
    blockhash: Option<[u8; 32]>,
    version: TransactionVersion,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            acct_keys: [ZERO_KEY; ACCT_MAX],
            acct_flags: [0; ACCT_MAX],
            acct_count: 0,
            instrs: [ZERO_INSTR; INSTR_MAX],
            instr_count: 0,
            instr_acct_buf: [0; MTU],
            instr_acct_len: 0,
            instr_data_buf: [0; MTU],
            instr_data_len: 0,
            fee_payer: None,
            blockhash: None,
            version: TransactionVersion::Legacy,
        }
    }

    pub fn version(&mut self, version: TransactionVersion) -> &mut Self {
        self.version = version;
        self
    }

    pub fn fee_payer(&mut self, pubkey: Pubkey) -> &mut Self {
        let idx = match self.find_account(&pubkey) {
            Some(i) => i,
            None => {
                assert!(self.acct_count < ACCT_MAX, "account table full");
                let i = self.acct_count;
                self.acct_keys[i] = pubkey;
                self.acct_flags[i] = 0;
                self.acct_count += 1;
                i
            }
        };
        self.acct_flags[idx] |= ACCT_F_SIGNER | ACCT_F_WRITABLE;
        self.fee_payer = Some(pubkey);
        self
    }

    pub fn recent_blockhash(&mut self, hash: [u8; 32]) -> &mut Self {
        self.blockhash = Some(hash);
        self
    }

    pub fn add_signer(&mut self, pubkey: Pubkey) -> Result<&mut Self, BuildError> {
        let idx = self.find_or_insert(pubkey)?;
        self.acct_flags[idx] |= ACCT_F_SIGNER;
        Ok(self)
    }

    pub fn add_instruction(&mut self, ix: &InstructionBuf) -> Result<&mut Self, BuildError> {
        if self.instr_count >= INSTR_MAX {
            return Err(BuildError::TooManyInstructions);
        }

        let prog_idx = self.find_or_insert(ix.program_id)?;

        let acct_off = self.instr_acct_len;
        for meta in ix.accounts {
            let idx = self.find_or_insert(meta.pubkey)?;
            if meta.is_signer {
                self.acct_flags[idx] |= ACCT_F_SIGNER;
            }
            if meta.is_writable {
                self.acct_flags[idx] |= ACCT_F_WRITABLE;
            }
            if self.instr_acct_len >= MTU {
                return Err(BuildError::DataTooLarge);
            }
            self.instr_acct_buf[self.instr_acct_len] = idx as u8;
            self.instr_acct_len += 1;
        }

        let data_off = self.instr_data_len;
        if self.instr_data_len + ix.data.len() > MTU {
            return Err(BuildError::DataTooLarge);
        }
        self.instr_data_buf[self.instr_data_len..self.instr_data_len + ix.data.len()]
            .copy_from_slice(ix.data);
        self.instr_data_len += ix.data.len();

        self.instrs[self.instr_count] = InstrEntry {
            program_idx: prog_idx as u8,
            acct_off: acct_off as u16,
            acct_cnt: ix.accounts.len() as u16,
            data_off: data_off as u16,
            data_len: ix.data.len() as u16,
        };
        self.instr_count += 1;

        Ok(self)
    }

    pub fn build(&self) -> Result<UnsignedTransaction, BuildError> {
        let fee_payer = self.fee_payer.ok_or(BuildError::NoFeePayer)?;
        let blockhash = self.blockhash.ok_or(BuildError::NoBlockhash)?;
        if self.instr_count == 0 {
            return Err(BuildError::NoInstructions);
        }

        let fp_idx = self
            .find_account(&fee_payer)
            .ok_or(BuildError::NoFeePayer)?;

        let mut sorted: [usize; ACCT_MAX] = core::array::from_fn(|i| i);
        let sorted = &mut sorted[..self.acct_count];
        let flags = &self.acct_flags;

        sorted.sort_by(|&a, &b| {
            let a_fp = (a == fp_idx) as u8;
            let b_fp = (b == fp_idx) as u8;
            b_fp.cmp(&a_fp)
                .then_with(|| acct_sort_key(flags[a]).cmp(&acct_sort_key(flags[b])))
                .then_with(|| a.cmp(&b))
        });

        let mut remap = [0u8; ACCT_MAX];
        for (new_idx, &old_idx) in sorted.iter().enumerate() {
            remap[old_idx] = new_idx as u8;
        }

        let mut sig_count: u8 = 0;
        let mut readonly_signed: u8 = 0;
        let mut readonly_unsigned: u8 = 0;

        for &old_idx in sorted.iter() {
            let f = self.acct_flags[old_idx];
            let signer = f & ACCT_F_SIGNER != 0;
            let writable = f & ACCT_F_WRITABLE != 0;
            if signer {
                sig_count += 1;
                if !writable {
                    readonly_signed += 1;
                }
            } else if !writable {
                readonly_unsigned += 1;
            }
        }

        if sig_count == 0 || sig_count as usize > sys::FD_TXN_SIG_MAX as usize {
            return Err(BuildError::TooManySigners);
        }

        let acct_count = self.acct_count;
        let mut buf = [0u8; MTU];
        let mut off: usize = 0;

        write_cu16(&mut buf, &mut off, sig_count as u16)?;

        let sig_off = off;
        let sigs_end = off + sig_count as usize * SIG_SZ;
        if sigs_end > MTU {
            return Err(BuildError::TransactionTooLarge);
        }
        off = sigs_end;

        let message_off = off;

        if self.version == TransactionVersion::V0 {
            if off >= MTU {
                return Err(BuildError::TransactionTooLarge);
            }
            buf[off] = 0x80;
            off += 1;
        }

        if off + 3 > MTU {
            return Err(BuildError::TransactionTooLarge);
        }
        buf[off] = sig_count;
        buf[off + 1] = readonly_signed;
        buf[off + 2] = readonly_unsigned;
        off += 3;

        write_cu16(&mut buf, &mut off, acct_count as u16)?;

        for &old_idx in sorted.iter() {
            if off + 32 > MTU {
                return Err(BuildError::TransactionTooLarge);
            }
            buf[off..off + 32].copy_from_slice(self.acct_keys[old_idx].as_bytes());
            off += 32;
        }

        if off + 32 > MTU {
            return Err(BuildError::TransactionTooLarge);
        }
        buf[off..off + 32].copy_from_slice(&blockhash);
        off += 32;

        write_cu16(&mut buf, &mut off, self.instr_count as u16)?;

        for i in 0..self.instr_count {
            let entry = &self.instrs[i];

            if off >= MTU {
                return Err(BuildError::TransactionTooLarge);
            }
            buf[off] = remap[entry.program_idx as usize];
            off += 1;

            write_cu16(&mut buf, &mut off, entry.acct_cnt)?;

            let acct_start = entry.acct_off as usize;
            let acct_end = acct_start + entry.acct_cnt as usize;
            if off + entry.acct_cnt as usize > MTU {
                return Err(BuildError::TransactionTooLarge);
            }
            for j in acct_start..acct_end {
                buf[off] = remap[self.instr_acct_buf[j] as usize];
                off += 1;
            }

            write_cu16(&mut buf, &mut off, entry.data_len)?;

            let data_start = entry.data_off as usize;
            let data_end = data_start + entry.data_len as usize;
            if off + entry.data_len as usize > MTU {
                return Err(BuildError::TransactionTooLarge);
            }
            buf[off..off + entry.data_len as usize]
                .copy_from_slice(&self.instr_data_buf[data_start..data_end]);
            off += entry.data_len as usize;
        }

        if self.version == TransactionVersion::V0 {
            write_cu16(&mut buf, &mut off, 0)?;
        }

        let len = off;
        validate(&buf, len)?;

        Ok(UnsignedTransaction {
            buf,
            len,
            sig_count,
            sig_off,
            message_off,
        })
    }

    fn find_account(&self, pubkey: &Pubkey) -> Option<usize> {
        (0..self.acct_count).find(|&i| self.acct_keys[i] == *pubkey)
    }

    fn find_or_insert(&mut self, pubkey: Pubkey) -> Result<usize, BuildError> {
        if let Some(i) = self.find_account(&pubkey) {
            return Ok(i);
        }
        if self.acct_count >= ACCT_MAX {
            return Err(BuildError::TooManyAccounts);
        }
        let idx = self.acct_count;
        self.acct_keys[idx] = pubkey;
        self.acct_flags[idx] = 0;
        self.acct_count += 1;
        Ok(idx)
    }
}

pub struct UnsignedTransaction {
    buf: [u8; MTU],
    len: usize,
    sig_count: u8,
    sig_off: usize,
    message_off: usize,
}

impl fmt::Debug for UnsignedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnsignedTransaction")
            .field("len", &self.len)
            .field("sig_count", &self.sig_count)
            .finish()
    }
}

impl UnsignedTransaction {
    pub fn set_signature(&mut self, index: usize, signature: &[u8; SIG_SZ]) {
        assert!(
            index < self.sig_count as usize,
            "signature index out of bounds"
        );
        let start = self.sig_off + index * SIG_SZ;
        self.buf[start..start + SIG_SZ].copy_from_slice(signature);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn message_bytes(&self) -> &[u8] {
        &self.buf[self.message_off..self.len]
    }

    pub fn signature_count(&self) -> usize {
        self.sig_count as usize
    }
}

fn acct_sort_key(flags: u8) -> u8 {
    match (flags & ACCT_F_SIGNER != 0, flags & ACCT_F_WRITABLE != 0) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
}

fn write_cu16(buf: &mut [u8; MTU], offset: &mut usize, val: u16) -> Result<(), BuildError> {
    if *offset + 3 > MTU {
        return Err(BuildError::TransactionTooLarge);
    }
    let len = unsafe { sys::fd_cu16_enc(val, buf[*offset..].as_mut_ptr()) } as usize;
    *offset += len;
    Ok(())
}

fn validate(buf: &[u8], len: usize) -> Result<(), BuildError> {
    let mut txn_buf = [0u8; sys::FD_TXN_MAX_SZ as usize];
    let result = unsafe {
        sys::fd_txn_parse_core(
            buf.as_ptr(),
            len as u64,
            txn_buf.as_mut_ptr() as *mut core::ffi::c_void,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };
    if result == 0 {
        Err(BuildError::ValidationFailed)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountCategory, Transaction};

    fn test_pubkey(n: u8) -> Pubkey {
        let mut bytes = [0u8; 32];
        bytes[0] = n;
        Pubkey::from_bytes(&bytes)
    }

    fn test_blockhash() -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = 0xAB;
        h[31] = 0xCD;
        h
    }

    #[test]
    fn test_build_minimal_legacy() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();

        assert_eq!(unsigned.signature_count(), 1);
        assert!(!unsigned.as_bytes().is_empty());
        assert!(!unsigned.message_bytes().is_empty());

        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();
        assert_eq!(txn.version(), TransactionVersion::Legacy);
        assert_eq!(txn.signature_count(), 1);
        assert_eq!(txn.instruction_count(), 1);
        assert_eq!(txn.account_count(), 2);
        assert_eq!(txn.fee_payer(), fee_payer);
        assert_eq!(*txn.recent_blockhash(), blockhash);
    }

    #[test]
    fn test_build_v0_minimal() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder
            .version(TransactionVersion::V0)
            .fee_payer(fee_payer)
            .recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();
        assert_eq!(txn.version(), TransactionVersion::V0);
        assert_eq!(txn.fee_payer(), fee_payer);
    }

    #[test]
    fn test_round_trip_with_data() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let dest = test_pubkey(3);
        let blockhash = test_blockhash();
        let ix_data = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[
                    AccountMeta {
                        pubkey: fee_payer,
                        is_signer: true,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: dest,
                        is_signer: false,
                        is_writable: true,
                    },
                ],
                data: &ix_data,
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();

        assert_eq!(txn.instruction_count(), 1);
        let instr = txn.instruction(0).unwrap();
        assert_eq!(instr.data(), &ix_data);
        assert_eq!(instr.account_count(), 2);
        assert_eq!(instr.program_id_address(), program);
    }

    #[test]
    fn test_account_ordering() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(10);
        let writable_signer = test_pubkey(2);
        let readonly_signer = test_pubkey(3);
        let writable_nonsigner = test_pubkey(4);
        let readonly_nonsigner = test_pubkey(5);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[
                    AccountMeta {
                        pubkey: readonly_nonsigner,
                        is_signer: false,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: writable_nonsigner,
                        is_signer: false,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: readonly_signer,
                        is_signer: true,
                        is_writable: false,
                    },
                    AccountMeta {
                        pubkey: writable_signer,
                        is_signer: true,
                        is_writable: true,
                    },
                ],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();

        let accounts = txn.accounts();
        assert_eq!(accounts[0], fee_payer);
        assert_eq!(accounts[1], writable_signer);
        assert_eq!(accounts[2], readonly_signer);

        let writable_signers: Vec<Pubkey> = txn
            .accounts_by_category(AccountCategory::WRITABLE_SIGNER)
            .map(|(_, pk)| pk)
            .collect();
        assert!(writable_signers.contains(&fee_payer));
        assert!(writable_signers.contains(&writable_signer));

        let readonly_signers: Vec<Pubkey> = txn
            .accounts_by_category(AccountCategory::READONLY_SIGNER)
            .map(|(_, pk)| pk)
            .collect();
        assert!(readonly_signers.contains(&readonly_signer));
    }

    #[test]
    fn test_account_dedup_permission_upgrade() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let shared = test_pubkey(3);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);

        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[AccountMeta {
                    pubkey: shared,
                    is_signer: false,
                    is_writable: false,
                }],
                data: &[1],
            })
            .unwrap();

        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[AccountMeta {
                    pubkey: shared,
                    is_signer: true,
                    is_writable: true,
                }],
                data: &[2],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();

        assert_eq!(txn.account_count(), 3);
        assert_eq!(txn.signature_count(), 2);
        assert!(
            txn.is_account_signer(txn.accounts().iter().position(|&a| a == shared).unwrap() as u16)
        );
        assert!(txn
            .is_account_writable(txn.accounts().iter().position(|&a| a == shared).unwrap() as u16));
    }

    #[test]
    fn test_multiple_instructions() {
        let fee_payer = test_pubkey(1);
        let program_a = test_pubkey(2);
        let program_b = test_pubkey(3);
        let acct = test_pubkey(4);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);

        builder
            .add_instruction(&InstructionBuf {
                program_id: program_a,
                accounts: &[AccountMeta {
                    pubkey: acct,
                    is_signer: false,
                    is_writable: true,
                }],
                data: &[0xAA],
            })
            .unwrap();

        builder
            .add_instruction(&InstructionBuf {
                program_id: program_b,
                accounts: &[AccountMeta {
                    pubkey: acct,
                    is_signer: false,
                    is_writable: true,
                }],
                data: &[0xBB, 0xCC],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();

        assert_eq!(txn.instruction_count(), 2);
        let i0 = txn.instruction(0).unwrap();
        let i1 = txn.instruction(1).unwrap();
        assert_eq!(i0.data(), &[0xAA]);
        assert_eq!(i1.data(), &[0xBB, 0xCC]);
        assert_eq!(i0.program_id_address(), program_a);
        assert_eq!(i1.program_id_address(), program_b);
    }

    #[test]
    fn test_program_id_is_readonly_nonsigner() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();

        let prog_idx = txn.accounts().iter().position(|&a| a == program).unwrap() as u16;
        assert!(!txn.is_account_signer(prog_idx));
        assert!(!txn.is_account_writable(prog_idx));
    }

    #[test]
    fn test_set_signature() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[],
                data: &[],
            })
            .unwrap();

        let mut unsigned = builder.build().unwrap();
        assert_eq!(unsigned.signature_count(), 1);

        let mut sig = [0u8; 64];
        sig[0] = 0xFF;
        sig[63] = 0xEE;
        unsigned.set_signature(0, &sig);

        let bytes = unsigned.as_bytes();
        assert_eq!(bytes[1], 0xFF);
        assert_eq!(bytes[64], 0xEE);
    }

    #[test]
    fn test_add_signer() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let extra_signer = test_pubkey(3);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder.add_signer(extra_signer).unwrap();
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[AccountMeta {
                    pubkey: extra_signer,
                    is_signer: true,
                    is_writable: false,
                }],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();
        assert_eq!(txn.signature_count(), 2);
    }

    #[test]
    fn test_fee_payer_always_index_zero() {
        let fee_payer = test_pubkey(99);
        let program = test_pubkey(1);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[AccountMeta {
                    pubkey: test_pubkey(2),
                    is_signer: true,
                    is_writable: true,
                }],
                data: &[],
            })
            .unwrap();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();
        assert_eq!(txn.fee_payer(), fee_payer);
        assert_eq!(txn.accounts()[0], fee_payer);
    }

    #[test]
    fn test_error_no_fee_payer() {
        let mut builder = TransactionBuilder::new();
        builder.recent_blockhash(test_blockhash());
        builder
            .add_instruction(&InstructionBuf {
                program_id: test_pubkey(1),
                accounts: &[],
                data: &[],
            })
            .unwrap();
        assert_eq!(builder.build().unwrap_err(), BuildError::NoFeePayer);
    }

    #[test]
    fn test_error_no_blockhash() {
        let mut builder = TransactionBuilder::new();
        builder.fee_payer(test_pubkey(1));
        builder
            .add_instruction(&InstructionBuf {
                program_id: test_pubkey(2),
                accounts: &[],
                data: &[],
            })
            .unwrap();
        assert_eq!(builder.build().unwrap_err(), BuildError::NoBlockhash);
    }

    #[test]
    fn test_error_no_instructions() {
        let mut builder = TransactionBuilder::new();
        builder
            .fee_payer(test_pubkey(1))
            .recent_blockhash(test_blockhash());
        assert_eq!(builder.build().unwrap_err(), BuildError::NoInstructions);
    }

    #[test]
    fn test_error_too_many_instructions() {
        let mut builder = TransactionBuilder::new();
        builder
            .fee_payer(test_pubkey(1))
            .recent_blockhash(test_blockhash());

        for i in 0..INSTR_MAX {
            builder
                .add_instruction(&InstructionBuf {
                    program_id: test_pubkey(2),
                    accounts: &[],
                    data: &[i as u8],
                })
                .unwrap();
        }

        let result = builder.add_instruction(&InstructionBuf {
            program_id: test_pubkey(2),
            accounts: &[],
            data: &[],
        });
        assert!(matches!(result, Err(BuildError::TooManyInstructions)));
    }

    #[test]
    fn test_message_bytes_for_signing() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let msg = unsigned.message_bytes();

        assert_eq!(msg[0], 1);
        assert_eq!(msg[1], 0);
        assert_eq!(msg[2], 1);
    }

    #[test]
    fn test_build_idempotent() {
        let mut builder = TransactionBuilder::new();
        builder
            .fee_payer(test_pubkey(1))
            .recent_blockhash(test_blockhash());
        builder
            .add_instruction(&InstructionBuf {
                program_id: test_pubkey(2),
                accounts: &[],
                data: &[1, 2, 3],
            })
            .unwrap();

        let first = builder.build().unwrap();
        let second = builder.build().unwrap();
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn test_instruction_account_addresses_match() {
        let fee_payer = test_pubkey(1);
        let program = test_pubkey(2);
        let acct_a = test_pubkey(3);
        let acct_b = test_pubkey(4);
        let blockhash = test_blockhash();

        let mut builder = TransactionBuilder::new();
        builder.fee_payer(fee_payer).recent_blockhash(blockhash);
        builder
            .add_instruction(&InstructionBuf {
                program_id: program,
                accounts: &[
                    AccountMeta {
                        pubkey: acct_a,
                        is_signer: false,
                        is_writable: true,
                    },
                    AccountMeta {
                        pubkey: acct_b,
                        is_signer: false,
                        is_writable: false,
                    },
                ],
                data: &[],
            })
            .unwrap();

        let unsigned = builder.build().unwrap();
        let txn = Transaction::parse(unsigned.as_bytes()).unwrap();
        let instr = txn.instruction(0).unwrap();
        let addrs = instr.account_addresses();
        assert_eq!(addrs[0], acct_a);
        assert_eq!(addrs[1], acct_b);
    }
}
