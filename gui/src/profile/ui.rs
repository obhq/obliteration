use super::{DisplayResolution, Profile};
use crate::ui::DisplaySettings;
use slint::{Model, ModelNotify, ModelRc, ModelTracker, SharedString, VecModel};
use std::any::Any;

/// Implementation of [`Model`] for [`crate::ui::Profile`].
pub struct ProfileModel {
    profiles: Vec<Profile>,
    resolutions: ModelRc<SharedString>,
    noti: ModelNotify,
}

impl ProfileModel {
    pub fn new(profiles: Vec<Profile>) -> Self {
        // Build resolution list.
        let resolutions = ModelRc::new(VecModel::from_iter(
            [
                DisplayResolution::Hd,
                DisplayResolution::FullHd,
                DisplayResolution::UltraHd,
            ]
            .into_iter()
            .map(|v| SharedString::from(v.to_string())),
        ));

        Self {
            profiles,
            resolutions,
            noti: ModelNotify::default(),
        }
    }
}

impl Model for ProfileModel {
    type Data = crate::ui::Profile;

    fn row_count(&self) -> usize {
        self.profiles.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.profiles.get(row).map(|p| crate::ui::Profile {
            name: p.name.clone().into(),
            display: DisplaySettings {
                resolution: p.display_resolution.to_string().into(),
                resolutions: self.resolutions.clone(),
            },
        })
    }

    fn set_row_data(&self, row: usize, data: Self::Data) {}

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.noti
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
