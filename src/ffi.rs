use crate::hoip::{PayloadM2S, PayloadS2M, PayloadType};
use crate::k_policy::KPolicySDMI;
use crate::{congestion_detection, NetworkModule};

type MasterNetworkModule =
    NetworkModule<PayloadM2S, PayloadS2M, congestion_detection::Window, KPolicySDMI>;
type SlaveNetworkModule =
    NetworkModule<PayloadS2M, PayloadM2S, congestion_detection::Window, KPolicySDMI>;

#[no_mangle]
pub unsafe extern "C" fn master_network_module_new() -> *mut MasterNetworkModule {
    let w = 0.1;
    let congestion_detector = congestion_detection::Window::new(5);
    let k_policy = KPolicySDMI {};
    let network_module = NetworkModule::<_, _, _, _>::new(
        "127.0.0.1:13380",
        "127.0.0.1:13370",
        congestion_detector,
        k_policy,
        w,
        10,
        PayloadType::Master,
    );
    Box::into_raw(Box::new(network_module))
}

#[no_mangle]
pub unsafe extern "C" fn master_network_module_send(
    network_module: *mut MasterNetworkModule,
    payload: PayloadM2S,
) {
    assert!(!network_module.is_null());
    let network_module = &mut *network_module;
    network_module.send(payload)
}

#[no_mangle]
pub unsafe extern "C" fn master_network_module_try_recv(
    network_module: *mut MasterNetworkModule,
    payload: *mut PayloadS2M,
) -> bool {
    assert!(!network_module.is_null());
    assert!(!payload.is_null());
    let network_module = &mut *network_module;
    let payload = &mut *payload;

    if let Some((_, received_payload)) = network_module.try_recv() {
        *payload = received_payload;
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn master_network_module_free(network_module: *mut MasterNetworkModule) {
    if !network_module.is_null() {
        Box::from_raw(network_module);
    }
}

#[no_mangle]
pub unsafe extern "C" fn slave_network_module_new() -> *mut SlaveNetworkModule {
    let w = 0.1;
    let congestion_detector = congestion_detection::Window::new(5);
    let k_policy = KPolicySDMI {};
    let network_module = NetworkModule::<_, _, _, _>::new(
        "127.0.0.1:13370",
        "127.0.0.1:13380",
        congestion_detector,
        k_policy,
        w,
        10,
        PayloadType::Master,
    );
    Box::into_raw(Box::new(network_module))
}

#[no_mangle]
pub unsafe extern "C" fn slave_network_module_send(
    network_module: *mut SlaveNetworkModule,
    payload: PayloadS2M,
) {
    assert!(!network_module.is_null());
    let network_module = &mut *network_module;
    network_module.send(payload);
}

#[no_mangle]
pub unsafe extern "C" fn slave_network_module_try_recv(
    network_module: *mut SlaveNetworkModule,
    payload: *mut PayloadM2S,
) -> bool {
    assert!(!network_module.is_null());
    assert!(!payload.is_null());
    let network_module = &mut *network_module;
    let payload = &mut *payload;

    if let Some((_, received_payload)) = network_module.try_recv() {
        *payload = received_payload;
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn slave_network_free(network_module: *mut SlaveNetworkModule) {
    if !network_module.is_null() {
        Box::from_raw(network_module);
    }
}
