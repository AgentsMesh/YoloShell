# Durable filesystem object identity

This package recognizes the same directory or regular file after reopening its
filesystem. It captures evidence from a borrowed descriptor and never opens a
caller-supplied pathname, writes an attribute, or changes a filesystem.

`DurableObjectIdentity::capture(fd)` records object kind, available birth time,
and one platform proof. `verify(fd)` captures the current proof and compares the
entire identity. Serialization has one current schema, bounded opaque handles,
and validation during deserialization. It does not read retired formats.

On Linux, Btrfs uses its filesystem UUID and kernel export handle. The opaque
handle includes the subvolume identity, so a normal Btrfs snapshot has a
different proof even when inode and birth time are copied. ext4, XFS and tmpfs
use the kernel filesystem UUID ioctl and export handles when available. A
kernel or filesystem without either capability is explicitly unsupported.
tmpfs evidence lasts for its filesystem instance; it does not promise that
tmpfs data survives a reboot. virtiofs has no supported backend here.

On macOS, APFS uses volume UUID, persistent file ID, and birth time obtained by
descriptor attributes. Requested attribute availability is checked explicitly.
Other Mac filesystems are unsupported until their guarantees are established.

Live device/mount numbers are used only to fence capture and are never stored in
the durable identity. Existing live descriptor, path, owner, mode, ACL, hardlink,
lock and namespace checks remain the caller's responsibility. A durable match
does not renew an old live lease or process authority, nor does it prove file
contents are unchanged. Durable file versions require an additional content
proof and a freshly captured live CAS identity under the appropriate lock.

A bitwise filesystem clone preserving both UUID and object evidence cannot be
distinguished using local filesystem evidence alone. This package does not
claim such external anti-clone authority. Actual reboot/remount acceptance and
consumer schema changes are separate from the foundation tests.
