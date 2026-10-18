//! Target registry managing probed targets and active target attachment.

use super::driver::TargetDriver;
use alloc::vec::Vec;

/// Target container holding target instances and active attachment index.
pub struct TargetRegistry<D: TargetDriver> {
    targets: Vec<D>,
    active_idx: Option<usize>,
}

impl<D: TargetDriver> TargetRegistry<D> {
    pub const fn new() -> Self {
        Self {
            targets: Vec::new(),
            active_idx: None,
        }
    }

    pub fn count(&self) -> usize {
        self.targets.len()
    }

    pub fn add(&mut self, target: D) {
        self.targets.push(target);
    }

    pub fn clear(&mut self) {
        if let Some(active) = self.active_mut() {
            active.detach();
        }
        self.active_idx = None;
        self.targets.clear();
    }

    pub fn get(&self, idx: usize) -> Option<&D> {
        self.targets.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut D> {
        self.targets.get_mut(idx)
    }

    pub fn active(&self) -> Option<&D> {
        self.active_idx.and_then(|idx| self.targets.get(idx))
    }

    pub fn active_mut(&mut self) -> Option<&mut D> {
        self.active_idx.and_then(|idx| self.targets.get_mut(idx))
    }

    pub fn attach(&mut self, idx: usize) -> Result<&mut D, super::driver::TargetError> {
        if idx >= self.targets.len() {
            return Err(super::driver::TargetError::NotAttached);
        }
        if let Some(prev) = self.active_mut() {
            prev.detach();
        }
        let target = &mut self.targets[idx];
        target.attach()?;
        self.active_idx = Some(idx);
        Ok(&mut self.targets[idx])
    }

    pub fn detach(&mut self) {
        if let Some(active) = self.active_mut() {
            active.detach();
        }
        self.active_idx = None;
    }
}
