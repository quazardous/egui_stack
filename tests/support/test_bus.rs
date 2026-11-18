//! A minimal bus for the tests: listeners in registration order, refusals, failures, and
//! fail-closed requests, as bus-contract describes them. No ordering constraints, deadlines,
//! threads or capabilities.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use bus_contract::{
    Altered, AnyInput, AnyListener, AnyLocalListener, AnyValue, Bus, CloneFn, Collected, Failure,
    FailureKind, FailureSink, FreshFn, HookError, ListenOpts, ListenerInfo, PendingRun,
    PendingWaker, PointInfo, PointKey, PointSpec, Refusal, Refused,
};

#[derive(Default)]
pub struct TestBus {
    declared: Mutex<BTreeMap<&'static str, PointSpec>>,
    listeners: Mutex<BTreeMap<&'static str, Vec<(String, AnyListener)>>>,
}

impl TestBus {
    /// The points declared, by id.
    pub fn declared(&self) -> Vec<&'static str> {
        self.declared.lock().unwrap().keys().copied().collect()
    }

    fn fail_closed(&self, point: &str) -> bool {
        self.declared
            .lock()
            .unwrap()
            .get(point)
            .is_some_and(|spec| spec.fail_closed)
    }
}

impl Bus for TestBus {
    fn declare_point(&self, point: PointKey, spec: PointSpec) -> Result<(), Refused> {
        let mut declared = self.declared.lock().unwrap();
        if declared.contains_key(point.id) {
            return Err(Refused::AlreadyDeclared { point: point.id });
        }
        declared.insert(point.id, spec);
        Ok(())
    }

    fn listen_any(
        &self,
        point: PointKey,
        id: String,
        _opts: ListenOpts,
        run: AnyListener,
    ) -> Result<(), Refused> {
        self.listeners
            .lock()
            .unwrap()
            .entry(point.id)
            .or_default()
            .push((id, run));
        Ok(())
    }

    fn listen_local_any(
        &self,
        point: PointKey,
        _id: String,
        _opts: ListenOpts,
        _run: AnyLocalListener,
    ) -> Result<(), Refused> {
        Err(Refused::TypeMismatch { point: point.id })
    }

    fn run_pending(&self) -> Vec<PendingRun> {
        Vec::new()
    }

    fn add_pending_waker(&self, _wake: PendingWaker) {}

    fn unlisten(&self, point: &str, id: &str) -> bool {
        let mut listeners = self.listeners.lock().unwrap();
        let Some(list) = listeners.get_mut(point) else {
            return false;
        };
        let before = list.len();
        list.retain(|(listener, _)| listener != id);
        list.len() != before
    }

    fn forget(&self, _owner: &str) -> usize {
        0
    }

    fn alter_any(
        &self,
        point: PointKey,
        input: AnyInput,
        mut value: AnyValue,
        clone: CloneFn,
    ) -> Altered<AnyValue> {
        let listeners: Vec<(String, AnyListener)> = self
            .listeners
            .lock()
            .unwrap()
            .get(point.id)
            .cloned()
            .unwrap_or_default();
        let fail_closed = self.fail_closed(point.id);
        let mut failures = Vec::new();
        let mut refused = None;
        for (id, run) in listeners {
            let mut edited = clone(&*value);
            match run(&*input, &mut *edited) {
                Ok(()) => value = edited,
                Err(HookError::Refuse(reason)) => {
                    refused = Some(Refusal::new(point.id, id, None, reason));
                    break;
                }
                Err(error) => {
                    let failure = Failure::new(
                        point.id,
                        id,
                        None,
                        FailureKind::Error(error.message().to_owned()),
                    );
                    if fail_closed {
                        refused = Some(Refusal::from_failure(&failure));
                        failures.push(failure);
                        break;
                    }
                    failures.push(failure);
                }
            }
        }
        let mut out = Altered::new(value);
        out.failures = failures;
        out.refused = refused;
        out
    }

    fn collect_any(
        &self,
        _point: PointKey,
        _input: AnyInput,
        _fresh: FreshFn,
    ) -> Collected<AnyValue> {
        Collected::new()
    }

    fn add_failure_sink(&self, _sink: FailureSink) {}

    fn points(&self) -> Vec<PointInfo> {
        Vec::new()
    }

    fn listeners(&self, point: &str) -> Option<Vec<ListenerInfo>> {
        self.listeners.lock().unwrap().get(point).map(|list| {
            list.iter()
                .map(|(id, _)| ListenerInfo::new(id.clone()))
                .collect()
        })
    }
}
