use super::MainWindow;
use crate::graphics::{GraphicsBuilder, PhysicalDevice};
use crate::profile::{CpuModel, DisplayResolution, Profile};
use config::ProductId;
use serde_bytes::ByteBuf;
use slint::{Model, ModelNotify, ModelTracker, SharedString, ToSharedString};
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::num::NonZero;
use std::rc::Rc;
use thiserror::Error;

/// Implementation of [`Model`] for [`PhysicalDevice`].
pub struct DeviceModel<G>(Rc<G>);

impl<G: GraphicsBuilder> DeviceModel<G> {
    pub fn new(g: Rc<G>) -> Self {
        Self(g)
    }

    pub fn position(&self, id: &[u8]) -> Option<i32> {
        self.0
            .physical_devices()
            .iter()
            .position(move |d| d.id() == id)
            .map(|i| i.try_into().unwrap())
    }

    pub fn get(&self, i: i32) -> Option<&impl PhysicalDevice> {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.0.physical_devices().get(i))
    }
}

impl<G: GraphicsBuilder> Model for DeviceModel<G> {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.0.physical_devices().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0
            .physical_devices()
            .get(row)
            .map(|d| SharedString::from(d.name()))
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Implementation of [`Model`] for [`DisplayResolution`].
pub struct ResolutionModel([DisplayResolution; 3]);

impl ResolutionModel {
    pub fn position(&self, v: DisplayResolution) -> Option<i32> {
        self.0
            .iter()
            .position(move |i| *i == v)
            .map(|v| v.try_into().unwrap())
    }

    pub fn get(&self, i: i32) -> Option<DisplayResolution> {
        usize::try_from(i).ok().and_then(|i| self.0.get(i)).copied()
    }
}

impl Default for ResolutionModel {
    fn default() -> Self {
        Self([
            DisplayResolution::Hd,
            DisplayResolution::FullHd,
            DisplayResolution::UltraHd,
        ])
    }
}

impl Model for ResolutionModel {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.0.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).map(|v| v.to_string().into())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Implementation of [`Model`] for [`CpuModel`].
pub struct CpuList([CpuModel; 3]);

impl CpuList {
    pub fn position(&self, v: CpuModel) -> Option<i32> {
        self.0
            .iter()
            .position(move |i| *i == v)
            .map(|v| v.try_into().unwrap())
    }

    pub fn get(&self, i: i32) -> Option<CpuModel> {
        usize::try_from(i).ok().and_then(|i| self.0.get(i)).copied()
    }
}

impl Default for CpuList {
    fn default() -> Self {
        Self([CpuModel::Host, CpuModel::Pro, CpuModel::ProWithHost])
    }
}

impl Model for CpuList {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.0.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).map(|v| v.to_shared_string())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Implementation of [`Model`] for [`ProductId`].
pub struct ProductList([ProductId; 4]);

impl ProductList {
    pub fn position(&self, v: ProductId) -> Option<i32> {
        self.0
            .iter()
            .position(move |i| *i == v)
            .map(|v| v.try_into().unwrap())
    }

    pub fn get(&self, i: i32) -> Option<ProductId> {
        usize::try_from(i).ok().and_then(|i| self.0.get(i)).copied()
    }
}

impl Default for ProductList {
    fn default() -> Self {
        Self([
            ProductId::DEVKIT,
            ProductId::TESTKIT,
            ProductId::USA,
            ProductId::SOUTH_ASIA,
        ])
    }
}

impl Model for ProductList {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.0.len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).map(|v| v.to_shared_string())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Implementation of [`Model`] for [`Profile`].
pub struct ProfileModel<G> {
    profiles: RefCell<Vec<Profile>>,
    devices: Rc<DeviceModel<G>>,
    resolutions: Rc<ResolutionModel>,
    cpus: Rc<CpuList>,
    products: Rc<ProductList>,
    noti: ModelNotify,
}

impl<G: GraphicsBuilder> ProfileModel<G> {
    pub fn new(
        profiles: Vec<Profile>,
        devices: Rc<DeviceModel<G>>,
        resolutions: Rc<ResolutionModel>,
        cpus: Rc<CpuList>,
        products: Rc<ProductList>,
    ) -> Self {
        Self {
            profiles: RefCell::new(profiles),
            devices,
            resolutions,
            cpus,
            products,
            noti: ModelNotify::default(),
        }
    }

    /// # Panics
    /// If `row` is not valid.
    pub fn select(&self, row: usize, dst: &MainWindow) {
        let profiles = self.profiles.borrow();
        let p = &profiles[row];

        dst.set_selected_device(self.devices.position(&p.display_device).unwrap_or(0));
        dst.set_selected_resolution(self.resolutions.position(p.display_resolution).unwrap());
        dst.set_selected_cpu(self.cpus.position(p.cpu_model).unwrap());
        dst.set_cpu_count(p.kernel_config.max_cpu.get().try_into().unwrap());
        dst.set_debug_address(p.debug_addr.to_shared_string());
        dst.set_selected_idps_product(
            self.products
                .position(p.kernel_config.idps.product)
                .unwrap(),
        );
        dst.set_idps_sub_product(slint::format!("{:#x}", p.kernel_config.idps.prodsub));
    }

    /// # Panics
    /// If `row` is not valid.
    pub fn update(&self, row: i32, src: &MainWindow) -> Result<RefMut<Profile>, ProfileError> {
        let row = usize::try_from(row).unwrap();
        let mut profiles = self.profiles.borrow_mut();
        let p = &mut profiles[row];
        let debug_addr = src
            .get_debug_address()
            .parse()
            .map_err(|_| ProfileError::InvalidDebugAddress)?;
        let idps_prodsub = src.get_idps_sub_product();
        let idps_prodsub = match idps_prodsub.strip_prefix("0x") {
            Some(v) => {
                u16::from_str_radix(v, 16).map_err(|_| ProfileError::InvalidIdpsSubProduct)?
            }
            None => idps_prodsub
                .parse()
                .map_err(|_| ProfileError::InvalidIdpsSubProduct)?,
        };

        p.display_device = ByteBuf::from(self.devices.get(src.get_selected_device()).unwrap().id());
        p.display_resolution = self.resolutions.get(src.get_selected_resolution()).unwrap();
        p.cpu_model = self.cpus.get(src.get_selected_cpu()).unwrap();
        p.kernel_config.max_cpu = src
            .get_cpu_count()
            .try_into()
            .ok()
            .and_then(NonZero::new)
            .unwrap();
        p.debug_addr = debug_addr;
        p.kernel_config.idps.product = self.products.get(src.get_selected_idps_product()).unwrap();
        p.kernel_config.idps.prodsub = idps_prodsub;

        Ok(RefMut::map(profiles, move |v| &mut v[row]))
    }

    pub fn into_inner(self) -> Vec<Profile> {
        self.profiles.into_inner()
    }
}

impl<G> ProfileModel<G> {
    pub fn push(&self, pf: Profile) -> i32 {
        let mut profiles = self.profiles.borrow_mut();
        let index = profiles.len();

        profiles.push(pf);
        self.noti.row_added(index, profiles.len());

        index.try_into().unwrap()
    }
}

impl<G: 'static> Model for ProfileModel<G> {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.profiles.borrow().len()
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.profiles
            .borrow()
            .get(row)
            .map(|p| p.name.to_shared_string())
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.noti
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Represents an error when [`ProfileModel::update()`] fails.
#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("invalid debug address")]
    InvalidDebugAddress,

    #[error("invalid IDPS sub-product")]
    InvalidIdpsSubProduct,
}
