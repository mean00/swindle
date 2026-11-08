//! Flashstub residency: the bookkeeping that decides whether the probe has to park code in
//! target RAM again.
//!
//! A flashstub is a small position independent program the probe writes into RAM and runs by
//! pointing the halted core at it. Uploading one costs hundreds of memory round trips, and the
//! flash manager asks for a `prepare` per write burst and per operation change, so the upload
//! must not happen more than once per session.
//!
//! The flash engine keeps no state of its own (it is rebuilt per call, see `engine.rs`), so the
//! flag lives in the driver, next to the capability mask describing what the uploaded images can
//! do. [`StubState::needs_upload`] is the whole protocol: covered operations keep the images,
//! anything else drops the flag because such a path may recycle the same RAM.

use super::driver::FlashOperation;

/// What a driver's flashstub images can do, as a bit set.
///
/// The mask is a property of the uploaded code, not of the operation an engine happens to ask
/// for: a set bit means "once uploaded, this entry point stays valid in RAM until the session
/// ends". Paths that do not use a stub at all (register pokes, controller block erase, the
/// half-word programming mode) have no bit and therefore always drop the flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StubCaps(u8);

impl StubCaps {
    /// No stub at all.
    pub const NONE: Self = Self(0);
    /// An erase stub sits in RAM (page or range erase, whatever the driver's image does).
    pub const ERASE: Self = Self(1 << 0);
    /// A programming stub sits in RAM.
    pub const WRITE: Self = Self(1 << 1);
    /// A CRC32 stub sits in RAM.
    ///
    /// Deliberately never advertised by a driver yet: the CH32V3 CRC stub borrows application
    /// SRAM and has to save/restore it, so it is not a resident capability.
    ///
    /// That is also the policy for the SRAM a stub borrows: a CRC stub saves the window it
    /// overwrites and puts it back (its result has to survive the run, and the range is
    /// short), while erase/write stubs do not -- their chunk is as big as what the flash
    /// manager writes per call, so stashing it would double exactly the traffic the stub path
    /// exists to avoid. A `load` rewrites that part of RAM anyway.
    pub const CRC32: Self = Self(1 << 2);

    /// Mask with everything in `self` and `other`.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Is every bit of `other` set here?
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// No capability at all?
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The capability `op` needs, [`StubCaps::NONE`] for operations that never use a stub.
    pub const fn for_operation(op: FlashOperation) -> Self {
        match op {
            FlashOperation::Erase | FlashOperation::MassErase => Self::ERASE,
            FlashOperation::Write => Self::WRITE,
            FlashOperation::None => Self::NONE,
        }
    }
}

/// Are this driver's flashstub images currently in target RAM?
///
/// Cheap enough to live in the driver cell (`u8` + `bool`). The flag is cleared when the driver
/// gives the controller back (`flash_done`) and when a stub run fails, because a failed run can
/// leave the core pointing at a half executed image and the next session must not assume it is
/// still there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct StubState {
    caps: StubCaps,
    loaded: bool,
}

impl StubState {
    /// State for a driver whose images advertise `caps`, with nothing uploaded yet.
    pub const fn new(caps: StubCaps) -> Self {
        Self {
            caps,
            loaded: false,
        }
    }

    /// A driver with no flashstub: every upload request answers `true`.
    pub const fn none() -> Self {
        Self::new(StubCaps::NONE)
    }

    /// The advertised capabilities.
    pub const fn caps(&self) -> StubCaps {
        self.caps
    }

    /// Are the images in RAM as far as this driver knows?
    pub const fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Does `op` require uploading the images?
    ///
    /// `true` when nothing is loaded, and also when `op` is not covered by what was uploaded:
    /// such a path may have reused the workspace, so the flag is dropped here and the caller
    /// ends up uploading again. [`FlashOperation::None`] (session start without an operation)
    /// never drops the flag.
    pub fn needs_upload(&mut self, op: FlashOperation) -> bool {
        let needed = StubCaps::for_operation(op);
        if !needed.is_empty() && !self.caps.contains(needed) {
            self.loaded = false;
        }
        !self.loaded
    }

    /// Record a successful upload.
    pub fn uploaded(&mut self) {
        self.loaded = true;
    }

    /// Forget the upload: a stub run failed, or the session handed the target back.
    pub fn invalidate(&mut self) {
        self.loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: StubCaps = StubCaps::ERASE.union(StubCaps::WRITE);

    #[test]
    fn caps_compose() {
        assert!(BOTH.contains(StubCaps::ERASE));
        assert!(BOTH.contains(StubCaps::WRITE));
        assert!(!BOTH.contains(StubCaps::CRC32));
        assert!(StubCaps::NONE.is_empty());
        // An empty mask is contained in everything, the way an empty set is a subset: that is
        // what makes `FlashOperation::None` a no-op for `needs_upload`.
        assert!(BOTH.contains(StubCaps::NONE));
        assert!(!StubCaps::NONE.contains(StubCaps::ERASE));
    }

    #[test]
    fn operations_map_to_their_capability() {
        assert_eq!(
            StubCaps::for_operation(FlashOperation::Erase),
            StubCaps::ERASE
        );
        assert_eq!(
            StubCaps::for_operation(FlashOperation::MassErase),
            StubCaps::ERASE
        );
        assert_eq!(
            StubCaps::for_operation(FlashOperation::Write),
            StubCaps::WRITE
        );
        assert_eq!(StubCaps::for_operation(FlashOperation::None), StubCaps::NONE);
    }

    /// One upload covers a whole session however often `prepare` runs (the manager asks once per
    /// operation change).
    #[test]
    fn one_upload_per_session() {
        let mut stubs = StubState::new(BOTH);
        assert!(stubs.needs_upload(FlashOperation::Write));
        stubs.uploaded();

        for _ in 0..32 {
            assert!(!stubs.needs_upload(FlashOperation::Write));
        }
        assert!(stubs.is_loaded());
    }

    /// A second session must not trust RAM the application may have overwritten.
    #[test]
    fn done_then_prepare_uploads_again() {
        let mut stubs = StubState::new(BOTH);
        assert!(stubs.needs_upload(FlashOperation::Write));
        stubs.uploaded();
        stubs.invalidate();
        assert!(stubs.needs_upload(FlashOperation::Write));
    }

    /// A path outside the advertised mask can recycle the workspace.
    #[test]
    fn uncovered_operation_invalidates() {
        let mut stubs = StubState::new(StubCaps::WRITE);
        assert!(stubs.needs_upload(FlashOperation::Write));
        stubs.uploaded();
        assert!(stubs.needs_upload(FlashOperation::Erase));
        assert!(!stubs.is_loaded());
    }

    /// Session start without an operation keeps whatever is loaded.
    #[test]
    fn no_operation_keeps_the_flag() {
        let mut stubs = StubState::new(BOTH);
        assert!(stubs.needs_upload(FlashOperation::None));
        stubs.uploaded();
        assert!(!stubs.needs_upload(FlashOperation::None));
        assert!(stubs.is_loaded());
    }

    /// A failed stub run clears the flag, so the next prepare uploads again.
    #[test]
    fn failed_run_forces_reupload() {
        let mut stubs = StubState::new(BOTH);
        stubs.uploaded();
        stubs.invalidate();
        assert!(stubs.needs_upload(FlashOperation::Write));
    }

    #[test]
    fn driver_without_stub_always_reports_upload() {
        let mut stubs = StubState::none();
        assert!(stubs.needs_upload(FlashOperation::Write));
        stubs.uploaded();
        assert!(stubs.needs_upload(FlashOperation::Write));
    }
}
