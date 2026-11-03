use yolo_capnp_fixture::{common_capnp::note, service_capnp::echo};

const TEXT: &str = "generated schemas preserve UTF-8: 你好";
const SEQUENCE: u64 = u64::MAX;

#[test]
fn authored_crate_root_can_include_generated_modules_without_a_build_script() {
    let mut message = capnp::message::Builder::new_default();
    message
        .init_root::<yolo_capnp_authored_fixture::authored_capnp::envelope::Builder<'_>>()
        .set_enabled(true);
    assert!(yolo_capnp_authored_fixture::enabled(&message));
}

#[test]
fn generated_schema_roundtrips_scalars_union_and_text() {
    let mut message = capnp::message::Builder::new_default();
    {
        let mut note = message.init_root::<note::Builder<'_>>();
        note.set_sequence(SEQUENCE);
        note.set_text(TEXT);
        note.set_complete(true);
    }
    let bytes = capnp::serialize::write_message_to_words(&message);
    let mut input = bytes.as_slice();
    let decoded = capnp::serialize::read_message(&mut input, Default::default()).unwrap();
    let note = decoded.get_root::<note::Reader<'_>>().unwrap();
    assert_eq!(note.get_sequence(), SEQUENCE);
    assert_eq!(note.get_text().unwrap().to_str().unwrap(), TEXT);
    assert!(matches!(note.which().unwrap(), note::Complete(true)));
    assert!(input.is_empty());
}

struct Echo;
impl echo::Server for Echo {
    async fn echo(
        self: capnp::capability::Rc<Self>,
        params: echo::EchoParams,
        mut results: echo::EchoResults,
    ) -> Result<(), capnp::Error> {
        results.get().set_note(params.get()?.get_note()?)?;
        Ok(())
    }
}

#[test]
fn generated_imports_and_rpc_capabilities_execute_a_real_local_call() {
    futures::executor::block_on(async {
        let client: echo::Client = capnp_rpc::new_client(Echo);
        let mut request = client.echo_request();
        {
            let mut note = request.get().init_note();
            note.set_sequence(SEQUENCE);
            note.set_text(TEXT);
            note.set_pending(());
        }
        let response = request.send().promise.await.unwrap();
        let note = response.get().unwrap().get_note().unwrap();
        assert_eq!(note.get_sequence(), SEQUENCE);
        assert_eq!(note.get_text().unwrap().to_str().unwrap(), TEXT);
        assert!(matches!(note.which().unwrap(), note::Pending(())));
    });
}
