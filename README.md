# canvas-native

Renders Obsidian canvases without Obsidian.

The binary draws a [JSON Canvas](https://jsoncanvas.org) file to SVG
or PNG with Obsidian's geometry and styling.

Canvas native provides a fast loop for agents to generate canvas json and
iterate visually.

```bash
cargo build --release
cp target/release/canvas-native ~/.local/bin/
canvas-native file.canvas out.png
```

`SKILL.md` teaches an agent to render after every edit and read the result.
Register this directory as an agent skill, or copy `SKILL.md` into your agent's
skill directory after putting `canvas-native` on your PATH.

## License

MIT. Inter is embedded under the SIL Open Font License 1.1; see
`assets/Inter-LICENSE.txt`. Obsidian is a trademark of Dynalist Inc., which
has no connection to this project.
