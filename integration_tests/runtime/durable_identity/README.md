# Isolated Linux remount acceptance

`driver` is a test-only executable. It does not add a Yolo product command.
Run each stage as the ordinary test user against one precreated private directory
on a fresh Btrfs image mounted inside a separate private mount namespace.

`seed ROOT` creates a Hub, its configuration, a Runtime socket registration,
and a TUI journal with an uncertain creation intent and confirmed pending receipt.
It records the public Hub identity, original live device number, directory
durable proofs, configuration state and exact private record bytes in a private
baseline. A private persistent socket root is on the same image. Deliberate
regular-file markers retain both certified namespaces when each helper exits;
their directory proofs and complete xattr certificate bytes must stay identical
before and after every remount and actual namespace admission. This prevents an
empty-namespace cleanup/recreation from accidentally satisfying acceptance.
No keys or private record contents are printed.

After the process exits and the filesystem is unmounted and mounted again at the
same pathname, `reopen ROOT` requires a changed `st_dev`, then opens the original
Hub, configuration, Runtime namespace and TUI creation journal. Identity, proofs,
record bytes and pending creation evidence must remain unchanged. Re-retaining
the identical creation intent must not grant a fresh Create attempt. An unchanged
live device number fails acceptance.

`reject-copy ROOT` checks a copied configuration and a replacement Hub directory
at the original pathname. Both must fail to acquire authority. The original Hub
directory is restored and its proof and records checked again; rejected copies
are retained inside this disposable fixture as evidence.

The image lives in the VM cache; its mountpoint uses a unique `/tmp/yr.XXXXXXXX`
directory so socket paths stay within the production length budget. The socket
root is its `s` child and remains part of the mounted persistent image.

The external mount harness must hold no helper process across unmount, isolate
mount propagation, validate the exact image/loop/mount relationships before
cleanup, and preserve test artifacts on failure. It must not reboot the VM,
change an existing filesystem mount, or touch installed services and user PTYs.
Durable reboot transaction phases use the existing dedicated integration test
binary, with its explicit fixture-root and phase environment variables.
