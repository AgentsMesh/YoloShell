use yolo_build_image::BuildImage;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_remote_startup_test_support::FOREIGN_DIGEST;

const IMAGE_SIZE_DELTA: u64 = 1;

#[derive(Clone, Copy, Debug)]
enum Role {
    Host,
    Hub,
    FileHelper,
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Digest,
    Size,
}

#[derive(Clone, Copy, Debug)]
pub struct ImageFault {
    role: Role,
    mutation: Mutation,
}

pub fn faults() -> impl Iterator<Item = ImageFault> {
    [Role::Host, Role::Hub, Role::FileHelper]
        .into_iter()
        .flat_map(|role| {
            [Mutation::Digest, Mutation::Size].map(|mutation| ImageFault { role, mutation })
        })
}

impl ImageFault {
    pub fn changed(self, original: &ServiceBuild) -> ServiceBuild {
        let mut build = original.clone();
        let image = match self.role {
            Role::Host => &mut build.host,
            Role::Hub => &mut build.hub,
            Role::FileHelper => &mut build.file_helper,
        };
        *image = match self.mutation {
            Mutation::Digest => {
                assert_ne!(image.sha256(), FOREIGN_DIGEST);
                BuildImage::new(FOREIGN_DIGEST.into(), image.size_bytes()).unwrap()
            }
            Mutation::Size => {
                BuildImage::new(image.sha256().into(), image.size_bytes() + IMAGE_SIZE_DELTA)
                    .unwrap()
            }
        };
        build
    }
}
