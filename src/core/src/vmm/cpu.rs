use std::error::Error;

/// Represents a core of the PS4 CPU.
pub trait Cpu {
    type GetStatesErr: Error;
    type SetStatesErr: Error;

    fn id(&self) -> usize;
    fn get_states(&mut self, states: &mut CpuStates) -> Result<(), Self::GetStatesErr>;
    fn set_states(&mut self, states: &CpuStates) -> Result<(), Self::SetStatesErr>;
}

/// States of [`Cpu`].
pub struct CpuStates {}
