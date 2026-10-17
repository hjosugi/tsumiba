# Data and sync

tsumiba keeps your things as **ordinary files in one folder**, on purpose: you can back
them up, sync them, grep them and open them in any editor, with or without tsumiba. `tsumiba
paths` prints where the folder is; [Configuration](configuration.md#where-things-are) has
the locations for each system.

## The layout

```text
<data folder>/
├── index.json                  pins, and what is known about each file
├── notes/
│   ├── 01a148d44c055be0439c.md     one Markdown file per note
│   └── …
├── files/
│   ├── 01a148d44c5b4e5d2364/       one folder per stored file
│   │   └── screenshot.png          …holding the file under its own name
│   └── …
├── trash/
│   └── 1760719443123_01a1…/        deleted items, kept 30 days
└── tsumiba.log
```

**Ids.** Every note and file has a 20-character id of lowercase hex digits: the creation
time in milliseconds (12 digits) followed by 8 random ones. Ids sort by creation time, and
the creation time can be read back out of one, so it is stored nowhere else.

**Notes** are `notes/<id>.md`, UTF-8, exactly what you typed. A note's *modified time* is
the file's modification time. Whether it is pinned is the only thing about a note that is
not in the file itself: that is in `index.json`.

**Files** each get their own folder `files/<id>/`, which is why two files called
`photo.png` never collide. For a *link*, nothing is stored there: the location is
remembered in `index.json`, and the original is untouched.

**`index.json`** holds the ids of pinned notes, and for each file its name, size, when it was
added, whether it is pinned, where it came from, and, for links, the original path. It is
written atomically (a new file is written and renamed over the old one), so a crash never
leaves half a file.

## It repairs itself

Everything that *can* be worked out from the folder is worked out from the folder. When
tsumiba starts, and whenever the shelf is opened, it looks at the folder again:

- a `notes/<id>.md` it did not know is **adopted** as a note;
- a note whose file is gone is dropped from the list;
- a note whose file changed (an editor, a sync tool) is **reloaded**;
- a `files/<id>/` folder it did not know is adopted, taking its name from the file inside;
- a stored file whose folder is gone is dropped;
- a link whose original has vanished stays on the shelf, marked **missing**.

So you can drop a `.md` file into `notes/` (name it with a valid id), or delete things from
the folder by hand, and the shelf follows.

If `index.json` is unreadable (or was written by a *newer* tsumiba than this one), tsumiba
moves it aside as `index.json.corrupt-<time>`, tells you, and rebuilds from the folder. You
lose only what lived in the index and nowhere else: which items were **pinned**, the
**source address** of files, and **links** (which exist only in the index). Notes and copied
files are all still there.

## The trash

Deleting a note, or removing a stored file, moves it to
`trash/<time>_<id>/` instead of destroying it. Each entry holds a `meta.json` (what it was)
and a `payload` (the note's text, or the file's folder). Entries older than **30 days** are
deleted when tsumiba starts. Removing a link leaves only a `meta.json`: there is no payload,
because the original was never touched.

The *Undo* button in the app restores an item straight away. To restore one by hand later:

- **a note:** move `payload` to `notes/<id>.md` (the id is in the entry's folder name,
  after the underscore);
- **a file:** move the `payload` folder to `files/<id>`. The shelf adopts it the next time
  it looks (pin and source are in `meta.json` if you want to note them).

Anything in `trash/` whose name tsumiba does not recognise is left alone.

## Syncing

Because notes are single files with unique names, the folder works with Syncthing, Dropbox,
iCloud Drive, a Git repository or a plain `rsync`. Some advice:

- **Sync `notes/` and `files/`; leave `index.json` out.** It is the one file two machines
  could both change, and a sync tool's conflict copy of it is ignored but not merged. Each
  machine then adopts what arrives (see above); you lose pins and sources across machines,
  nothing else.
- **Changes show up when the shelf is opened**, not live while it is already open.
- **Editing one note on two machines at once** is the usual sync conflict: the tool keeps one
  version, or makes a copy under a different name. A copy whose name is not a valid id is not
  a note, so it is ignored. Find it in the folder and merge by hand.
- Do not sync the configuration folder between machines, or at least not `token` and
  `server.json`: each machine should have its own token.

Point a machine at a folder of your choice with `TSUMIBA_DATA_DIR` (or
[move everything](configuration.md#environment-variables) with `TSUMIBA_HOME`). Put the
variable in your login environment so the app, the command line and the autostart entry all
see it.

## Backing up

Copy the data folder (and, if you like, `config.toml`). Do it while tsumiba is idle or
quit, to avoid copying a note mid-write. Restoring is copying it back; the shelf adopts what
it finds.

## Removing everything

Quit tsumiba, then delete the data and configuration folders. That is all it ever wrote
(plus the login entry, if you turned on *Start at login*).
