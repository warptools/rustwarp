Warpforge Depot
===============

The "Depot" is the part of your filesystem where Warpforge keeps unpacked filesets,
indexed by WareID, ready to use.
It also includes caching of packed filesets.

As with everything that focuses on WareIDs,
the Depot keeps all data organized in a content-addressed way.
The Depot does not handle any kinds of human-readable or mutable name associations to that data.
(Look to "catalogs" and other higher-level parts of Warpforge for name-association features.)

Mostly, the Depot is used indirectly.
When using Warpforge to evaluating formulas in containers,
that process will use the Depot to stage and unpack any input filesystems,
and store any output filesystems to the Depot as well.

It is also possible, though less common, to interact with the Depot directly,
via the `warpforge depot` family of commands.


Key Features
------------

- Stores filesets, indexed by WareID.
- Makes those filesets "ready to use" (e.g. ready to mount, or even read directly).
- Caches packed wares when applicable (e.g. during downloading).
- Supports multiple pack systems (tar+hash, git, ociimg, and more).


Depot Internals
---------------

### directory layout example

Sometimes an example makes things clear fast, so let's start with that, and then explain it later ;)

```
depot/1000/riotar/unpack/{...cas...}
depot/1000/riotar/pack/{...cas...}.tgz
depot/1000/riotar/xform/{... symlink of pack hash to resultant unpack hash ...}
depot/1000/git/repos/{...cas...}
depot/1000/git/worktrees/{...cas...}
depot/1000/ociimg/presentation/{...cas...}
depot/staging/{...uuid...} // mounted into ersatz for live output gathering!
```

You probably have questions.

- Why "pack" and "unpack" directories?
	- -> Jump down to [#presentation-dirs-vs-packed-caches](the presentation dirs vs packed caches).
- Why the "git" vs "riotar" vs {etc} directories?
	- Because each type of packing has its own hashing strategy, so they need to be namespaced.
	- And because [#pack-systems-are-plugins](pack systems are plugins), and some have their own individual interesting internal implementation details
	  (for example, oci images have layer dirs; git has its own internal object sharding; etc).
- What's that "1000" for?
	- -> Jump down to [#ownership-and-privileges-vs-caching](the section on ownership and privileges vs caching).
- "Staging"?
	- -> Jump down to [#output-gathering](the section on output gathering).

### practicality and safety

The Depot design and data layout is designed to be cautious and practical but not paranoid.
The design criteria are evaluated in the following order, from strongest influence to best-effort:

- the data layout must be possible to create, access, and maintain without any escalated privileges,
  nor use of advanced techniques such as namespaces.
- the data layout must be usable without performance problems.
- the data layout must be content addressed, so that any kind of "cache collision" is a non-issue.
- the data layout must resist accidental mutation that would cause the cache to
  be inaccurate to its content-addressed primary key.

What this means in practice is:

- For the most part, filesets are unpacked completely on the regular filesystem
  (for speed of access -- as contrasted to FUSE mounts or other indirections).
- The Depot code resists using chroot or namespaces or containers (even if those would make immutability and other things easier).
- Mutation resistance is mostly in the hands of the user.
	- This is something that is totally covered when Warpforge is using the Depot:
	  the rest of Warpforge *does* use containers, and will use read-only mounts to make sure accidental mutations of the Depot is impossible.
	- If you're touching the Depot filesystem yourself directly, you must be well behaved,
	  or any cache corruption issues are on you.
	- This is unfortunate, but a direct consequence of prioritizing low-priv operation over corruption resistance.
	  (Even techniques like setting file modes to `0444` are unavailable to us, because that would be visible to,
	  and potentially affect, user workloads that inspect these files.)

### presentation dirs vs packed caches

The Depot may contain several related forms of any fileset it stores:
it can be packed (e.g. in a tarball), or unpacked (a plain dir),
and the same data can be kept in both forms when convenient.

There's (almost) always an unpacked dir for each WareID that's "ready to use":
that is, it's got all the content, and it's in regular files...
so that there's no further processing to do,
and speed of access predictable and is that of the normal native filesystem.
One of the main APIs of the Depot is to ask it for a presentation dir of a WareID.

"Almost"?  Well, [#pack-systems-are-plugins](pack systems are plugins).
Some of them do have optional modes of operations where a fully prepared presentation dir _isn't_ made,
and some amount of mounting is required in order to get a usable filesystem.
In these cases, asking the Depot for a presentation dir will result in either getting an error,
or if you've indicated you can accept it, receiving a mount spec as a result instead.
(This is speculative.  No current pack system implementations actually do this.)

### pack systems are plugins

It's the Depot's job to cache and present data from multiple different pack systems
(e.g. the part before the colon in a WareID).

All of these have some level of support:

- tar (plus a custom hashing strategy)
- git
- ociimg

The main criteria for ending up in this list is:

- Does it have a hash?  (For git and oci images, the answer is "yes", so that was easy.  For tar, this is why we had to invent one.)
- Is it useful?  (We've found all of the examples above useful in common workflows.)

There's some amount of code required in the Depot for understanding and integrating each one of these.
(In particular, there's usually some code for understanding exactly which metadata are relevant,
because this can affect cache maintenance in the context of the rest of Warpforge.)

It's possible for the list of supported pack systems to grow.
We're likely to be somewhat conservative about this,
because more things to support is, well, more things to support ;)
and having a larger diversity of pack formats is an influence that makes the ecosystem of users more fractured,
which is not necessarily helpful (e.g., is this git hash and this tar hash representing the same files?  Maybe, but it's costly to check!).
But if something is overwhelmingly useful, the Depot design is ready to extend to embrace it.

### output gathering

The Depot filesystem layout has a "staging" directory which can be used when creating new filesets.

It's assumed that the entire Depot directory forest is on one host filesystem,
and when Warpforge is producing new files, we want them to be on the same filesystem
as where the Depot will store them after the process completes.
So, we have Warpforge mount directories from under the "staging" path into its containers
for collecting output files, and then when we're done, moving them into the Depot for safekeeping is faster.

(The exact details of how it's "faster" may vary by pack system implementation.
For example, some will use hardlinks.  Others may move files.
Both of these basic operations are faster when they don't cross a filesystem boundary!)

### where does it all go?

By default, in the XDG_CONFIG_CACHE directory.

Warpforge workspace config can also specify where the Depot for that workspace should go (todo:future).

See the 'warpforge-cfg' crate for the details of where this is determined.



Errata
------

### comparable concepts

Conceptually similar features occur in other projects as well:
for example, Nix and Guix have a similar feature
as part of their package managers which is called the "store",
which has the job of caching many filesets while treating them independently.

In contrast to the Depot system:
those two have exactly one pack system and identification system, whereas the Depot supports several;
and the exactly one pack system they support has no support for ownership nor most other metadata.
Those two also include parts of package names in their internal filesystem,
meaning they're not actually content addressed.
(The hash used in those systems is also often not actually a content hash, though it is often assumed to be.)
So, overall, despite the abstract conceptual similarity, the divergences are considerable.

### ownership and privileges vs caching

Filesystems are complicated and can contain many different kinds of metadata.
Some of these metadata are further complicated by requiring privileges to change them
(such as ownership and groups on unix-like filesystems).

The Depot system is designed to support data organized by several different systems,
some of which support these kinds of tricky metadata, and some of which... don't!

For systems that ignore uid+gid issues (and are therefore much simpler):
the Depot keeps content in numeric directories, based on the uid that Warpforge
was running with at the time.
(In most cases then, this will be your user ID, and there will probably only be one such directory.)

These numeric directories have the following properties:

- All files and folders within them will have exactly that numeric uid and gid.
- There will be no other uids or gids.
- All files and folders created within them will have been created in a way that was possible without escalated privledges of any kind.
	- That means: no exciting device nodes; no exciting xattrs; etc.

These numeric directories keep things from becoming problematic if
the same Depot directory root was used by multiple users.
(In one particular common case, it's possible to do this accidentally when using sudo.)

For filesets that _do_ contain more tricky metadata...
a different set of directories is used, which involves encoding some properties
about which of those metadata are in play into a more complex directory name.

The numeric dirs do most of the heavy lifting in practice most of the time...
because they're what's available when running containers without privileges
(and without setting up uid/gid-mapping namespaces).

### symlinks and security

Symlinks are allowed in filesets handled by the Depot.
(It wouldn't be a very useful system if they weren't!)
Several of the supported pack systems also support symlinks.

Symlinks can point at any target path they want.
That can include absolute paths (again, it wouldn't be a very useful system if they couldn't!),
and that can also include paths that don't exist (which is a rare but totally valid thing to do).
Symlinks are, both effectively and in literal implementation, simply a container of a string.

The reason we need to talk about symlinks in the context of the Depot and for reasons of security is
that naively written code using the kernel filesystem APIs will traverse symlinks silently.
This could result in unpacker code following a pack's directive to create a symlink...
and then subsequently unpacking another file "beyond" that symlink,
which could then point *outside* the Depot directories, creating a security problem.

By example: Unpacking a fileset with a symlink at "./a" that points to "./b" is fine.
But if the format we're unpacking from subsequently tries to describe a file which
shall be unpacked at the path "./a/c", _that_ would be dangers.
We need that operation to be invalid and cause unpacking to stop,
and cause that pack to be reported as invalid.

So, to be correct and safe, Depot file unpackers must refuse to traverse any symlinks during unpack operations.

It's difficult to provide guardrails for this without using escalated privledges.
(A chroot would certainly do it, but we want the Depot code to be able to run without such drastic requirements!)
Therefore, this security property has to be maintained mainly by
making an effort to vet this property when integrating a pack system with the Depot.

If you'd like to implement code for a pack system and want to make sure it does the right thing with symlinks,
consider using the "libpathrs" crate.  It provides an easy-to-use facade over kernel APIs for the filesystem
which directly instruct the kernel itself not to resolve symlinks, making the secure and desired behavior easy.
