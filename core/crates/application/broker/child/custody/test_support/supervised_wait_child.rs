use yolo_process::DetachedChildCustody;

pub(crate) struct OwnedChild {
    pub(crate) custody: DetachedChildCustody,
}

impl OwnedChild {
    pub(super) fn capture(child: std::process::Child) -> anyhow::Result<Self> {
        match DetachedChildCustody::capture(child) {
            Ok(custody) => Ok(Self { custody }),
            Err(error) => {
                let detail = error.to_string();
                let _cleanup = Self {
                    custody: error.into_custody(),
                };
                Err(anyhow::Error::msg(detail))
            }
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.custody.request_direct_termination();
        let _ = self.custody.wait();
    }
}
