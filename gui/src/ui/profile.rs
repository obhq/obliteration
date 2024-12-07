use super::MainWindow;
use crate::profile::Profile;
use slint::{Model, ModelNotify, ModelTracker, SharedString};
use std::any::Any;
use std::cell::{RefCell, RefMut};

/// Implementation of [`Model`] for [`Profile`].
pub struct ProfileModel {
    profiles: RefCell<Vec<Profile>>,
    noti: ModelNotify,
}

impl ProfileModel {
    pub fn new(profiles: Vec<Profile>) -> Self {
        Self {
            profiles: RefCell::new(profiles),
            noti: ModelNotify::default(),
        }
    }

    /// # Panics
    /// If `row` is not valid.
    pub fn select(&self, row: usize, dst: &MainWindow) {}

    /// # Panics
    /// If `row` is not valid.
    pub fn update(&self, row: usize, src: &MainWindow) -> RefMut<Profile> {
        let mut profiles = self.profiles.borrow_mut();

        RefMut::map(profiles, move |v| &mut v[row])
    }
}

impl Model for ProfileModel {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.profiles.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.profiles.borrow().get(row).map(|p| p.name().into())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.noti
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
