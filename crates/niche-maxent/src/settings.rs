//! MaxEnt training settings. Defaults reproduce `density/parameters.csv` of MaxEnt 3.4.4.

/// Feature classes that may be used. With `auto_features`, classes are further restricted
/// by the number of training samples (see [`MaxentSettings::hinge_threshold`] etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureClasses {
    pub linear: bool,
    pub quadratic: bool,
    pub product: bool,
    pub threshold: bool,
    pub hinge: bool,
}

impl Default for FeatureClasses {
    fn default() -> Self {
        // MaxEnt 3.4.4 defaults: threshold features are off.
        Self {
            linear: true,
            quadratic: true,
            product: true,
            threshold: false,
            hinge: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaxentSettings {
    pub features: FeatureClasses,
    /// `autofeature`: restrict feature classes by sample count.
    pub auto_features: bool,
    /// `betamultiplier`: multiplies every automatic regularization parameter.
    pub beta_multiplier: f64,
    /// Overrides of the automatic betas (`beta_lqp`, `beta_categorical`, `beta_threshold`,
    /// `beta_hinge`); `None` keeps the automatic value.
    pub beta_lqp: Option<f64>,
    pub beta_categorical: Option<f64>,
    pub beta_threshold: Option<f64>,
    pub beta_hinge: Option<f64>,
    pub max_iterations: usize,
    pub convergence_threshold: f64,
    /// `addsamplestobackground`: add presences whose environmental combination is absent
    /// from the background.
    pub add_samples_to_background: bool,
    /// `addallsamplestobackground`
    pub add_all_samples_to_background: bool,
    pub l2lq_threshold: usize,
    pub lq2lqpt_threshold: usize,
    pub hinge_threshold: usize,
    /// Hidden MaxEnt parameter `parallelUpdateFrequency`.
    pub parallel_update_frequency: usize,
}

impl Default for MaxentSettings {
    fn default() -> Self {
        Self {
            features: FeatureClasses::default(),
            auto_features: true,
            beta_multiplier: 1.0,
            beta_lqp: None,
            beta_categorical: None,
            beta_threshold: None,
            beta_hinge: None,
            max_iterations: 500,
            convergence_threshold: 1e-5,
            add_samples_to_background: true,
            add_all_samples_to_background: false,
            l2lq_threshold: 10,
            lq2lqpt_threshold: 80,
            hinge_threshold: 15,
            parallel_update_frequency: 30,
        }
    }
}
