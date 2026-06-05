//! SPATIAL ROLE: THE PAYMENTS SEAM — a clean, swappable interface for charging a customer
//! and paying out a driver. This PR ships ONLY the interface + a no-op default so a real
//! provider (e.g. GoDaddy Payments) drops in later WITHOUT touching auth, jobs, or
//! setup.sh. No fake charges are performed: the no-op reports "unconfigured".

use std::fmt;

/// The outcome of attempting to settle a completed job.
#[derive(Debug, Clone)]
pub enum Settlement {
    /// No provider configured — the seam is present but inert (this PR).
    Unconfigured,
    /// A real provider charged the customer and queued the driver payout.
    Settled { reference: String },
    /// A real provider declined / errored.
    Failed { reason: String },
}
impl fmt::Display for Settlement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Settlement::Unconfigured => write!(f, "unconfigured"),
            Settlement::Settled { reference } => write!(f, "settled:{reference}"),
            Settlement::Failed { reason } => write!(f, "failed:{reason}"),
        }
    }
}

/// What a completed job hands the payments provider. Identity comes from authenticated
/// accounts (customer/driver), which is exactly why real accounts exist (see auth.rs).
#[derive(Debug, Clone)]
pub struct Charge {
    pub job_id: i64,
    pub customer_id: i64,
    pub driver_id: i64,
    pub amount_cents: i64,
    pub memo: String,
}

/// The swap point. A future GoDaddy-backed implementor replaces `NoopProvider` with no
/// changes to calling code.
pub trait PaymentProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn settle(&self, charge: &Charge) -> Settlement;
}

/// The default in this PR: present and wired at the job-completion seam, but inert.
#[derive(Debug, Default, Clone)]
pub struct NoopProvider;
impl PaymentProvider for NoopProvider {
    fn name(&self) -> &'static str { "noop" }
    fn settle(&self, _charge: &Charge) -> Settlement { Settlement::Unconfigured }
}
