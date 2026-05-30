use std::collections::HashMap;

pub trait Service {
    fn start(self: Box<Self>) -> Result<Box<dyn RunningService>, String>;
}

pub trait RunningService {
    fn healthcheck(&self) -> Result<(), String> {
        Ok(())
    }

    fn env(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

pub struct Services {
    running: Vec<Box<dyn RunningService>>,
    env: HashMap<String, String>,
}

impl Services {
    pub fn start(services: Vec<Box<dyn Service>>) -> Result<Self, String> {
        let mut running = Vec::with_capacity(services.len());
        let mut env = HashMap::new();
        for service in services {
            let handle = service.start()?;
            handle.healthcheck()?;
            env.extend(handle.env());
            running.push(handle);
        }
        Ok(Self { running, env })
    }

    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    pub fn len(&self) -> usize {
        self.running.len()
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct DummyService {
        dropped: Arc<Mutex<bool>>,
    }

    impl Service for DummyService {
        fn start(self: Box<Self>) -> Result<Box<dyn RunningService>, String> {
            Ok(Box::new(DummyRunningService {
                dropped: self.dropped.clone(),
            }))
        }
    }

    struct DummyRunningService {
        dropped: Arc<Mutex<bool>>,
    }

    impl RunningService for DummyRunningService {
        fn env(&self) -> HashMap<String, String> {
            HashMap::from([("DUMMY_URL".to_string(), "dummy://ready".to_string())])
        }
    }

    impl Drop for DummyRunningService {
        fn drop(&mut self) {
            *self.dropped.lock().expect("dummy drop mutex poisoned") = true;
        }
    }

    #[test]
    fn services_start_healthcheck_and_collect_env() {
        let dropped = Arc::new(Mutex::new(false));
        let services = Services::start(vec![Box::new(DummyService {
            dropped: dropped.clone(),
        })])
        .expect("dummy service starts");
        assert_eq!(services.len(), 1);
        assert_eq!(
            services.env().get("DUMMY_URL").map(String::as_str),
            Some("dummy://ready")
        );
        assert!(!*dropped.lock().expect("dummy drop mutex poisoned"));
        drop(services);
        assert!(*dropped.lock().expect("dummy drop mutex poisoned"));
    }
}
