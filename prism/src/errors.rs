use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuContextError {
    #[error("Impossible de créer la surface d'affichage WGPU : {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),

    #[error("Aucun adaptateur GPU compatible n'a été trouvé sur cette machine")]
    AdapterNotFound(#[from] wgpu::RequestAdapterError),

    #[error("Échec lors de la récupération du périphérique GPU (Device) : {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),
}