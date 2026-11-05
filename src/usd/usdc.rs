//! USD binary layers (`.usdc`, Pixar's "crate" format), written as they go:
//! array values stream to the output the moment a prim hands them over, so a
//! scene's meshes never have to sit beside a text copy of themselves. The
//! structure (tokens, paths, specs, fields) is small and written at the end;
//! the header at the start is patched last. Layout in
//! `docs/octane/usd.md#usdc-crate-layout`.

use super::*;

/// The crate version written: Pixar's default for new files.
const VERSION: [u8; 3] = [0, 8, 0];

/// Integer arrays shorter than this are written plainly (as Pixar does).
const MIN_COMPRESSED: usize = 16;

/// LZ4's largest block input (`LZ4_MAX_INPUT_SIZE`); longer data goes in
/// chunks of this size.
const LZ4_MAX_INPUT: u64 = 0x7E00_0000;

/// Spec types (`SdfSpecType`).
const SPEC_ATTRIBUTE: u32 = 1;
const SPEC_PRIM: u32 = 6;
const SPEC_PSEUDO_ROOT: u32 = 7;
const SPEC_RELATIONSHIP: u32 = 8;

/// Value types (`crateDataTypes.h`).
#[derive(Clone, Copy)]
#[repr(u8)]
enum Ty {
    Bool = 1,
    Int = 3,
    Float = 8,
    Double = 9,
    String = 10,
    Token = 11,
    Matrix4d = 15,
    Quath = 18,
    Vec2f = 20,
    Vec3f = 24,
    TokenListOp = 32,
    PathListOp = 34,
    TokenVector = 41,
    Specifier = 42,
    Variability = 44,
}

/// A value's 64-bit file representation (`ValueRep`): type, flags and
/// either the value itself (inlined) or its offset in the file.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Rep(u64);

impl Rep {
    fn inline(t: Ty, payload: u32) -> Rep {
        Rep(1 << 62 | (t as u64) << 48 | payload as u64)
    }
    fn at(t: Ty, offset: u64) -> Rep {
        Rep((t as u64) << 48 | offset)
    }
    fn array(t: Ty, offset: u64, compressed: bool) -> Rep {
        Rep(1 << 63 | if compressed { 1 << 61 } else { 0 } | (t as u64) << 48 | offset)
    }
}

/// An output a layer can be written to: sequential writes, plus one patch
/// of bytes already written (the header, once the table of contents is).
pub trait Patch: Write {
    /// Overwrite `data` at byte `at` of what was written.
    fn patch(&mut self, at: u64, data: &[u8]) -> std::io::Result<()>;
}

impl Patch for std::io::Cursor<Vec<u8>> {
    fn patch(&mut self, at: u64, data: &[u8]) -> std::io::Result<()> {
        let at = at as usize;
        self.get_mut()[at..at + data.len()].copy_from_slice(data);
        Ok(())
    }
}

impl<W: Write + std::io::Seek> Patch for crate::orbx::OrbxWriter<'_, W> {
    fn patch(&mut self, at: u64, data: &[u8]) -> std::io::Result<()> {
        self.patch_current(at, data)
    }
}

/// A prim being written: its path, fields so far, children and properties.
struct OpenPrim {
    path: String,
    fields: Vec<(u32, Rep)>,
    children: Vec<u32>,
    properties: Vec<u32>,
}

/// Writes one `.usdc` layer through `LayerSink`.
pub(super) struct UsdcWriter<'a, W: Patch> {
    out: &'a mut W,
    /// Bytes written so far: the next value's offset.
    pos: u64,
    tokens: Vec<String>,
    token_index: HashMap<String, u32>,
    /// Strings: each a token index.
    strings: Vec<u32>,
    string_index: HashMap<u32, u32>,
    /// Every path, as text, with its index.
    paths: Vec<String>,
    path_index: HashMap<String, u32>,
    fields: Vec<(u32, Rep)>,
    field_index: HashMap<(u32, Rep), u32>,
    /// Field sets, each ended by `u32::MAX`.
    field_sets: Vec<u32>,
    /// Specs: path, field set, spec type.
    specs: Vec<(u32, u32, u32)>,
    stack: Vec<OpenPrim>,
}

impl<'a, W: Patch> UsdcWriter<'a, W> {
    /// Start a layer with the stage metadata ORBIS writes: Y up, meters,
    /// `Scene` as the default prim.
    pub(super) fn new(out: &'a mut W) -> Result<Self> {
        out.write_all(&[0u8; 88])?; // the header, patched by `finish`
        let mut w = UsdcWriter {
            out,
            pos: 88,
            tokens: Vec::new(),
            token_index: HashMap::new(),
            strings: Vec::new(),
            string_index: HashMap::new(),
            paths: Vec::new(),
            path_index: HashMap::new(),
            fields: Vec::new(),
            field_index: HashMap::new(),
            field_sets: Vec::new(),
            specs: Vec::new(),
            stack: Vec::new(),
        };
        // Token 0 is never a property name: the paths section marks property
        // names by negating their index.
        w.token("");
        let mut root = OpenPrim { path: "/".into(), fields: Vec::new(), children: Vec::new(), properties: Vec::new() };
        root.fields.push((w.token("defaultPrim"), w.token_rep("Scene")));
        root.fields.push((w.token("metersPerUnit"), Rep::inline(Ty::Double, 1.0f32.to_bits())));
        root.fields.push((w.token("upAxis"), w.token_rep("Y")));
        root.fields.push((w.token("documentation"), w.string_rep("Exported by ORBIS")));
        w.stack.push(root);
        Ok(w)
    }

    fn token(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.token_index.get(s) {
            return i;
        }
        let i = self.tokens.len() as u32;
        self.tokens.push(s.to_string());
        self.token_index.insert(s.to_string(), i);
        i
    }

    fn token_rep(&mut self, s: &str) -> Rep {
        Rep::inline(Ty::Token, self.token(s))
    }

    fn string_rep(&mut self, s: &str) -> Rep {
        let t = self.token(s);
        let next = self.strings.len() as u32;
        let i = *self.string_index.entry(t).or_insert(next);
        if i == next {
            self.strings.push(t);
        }
        Rep::inline(Ty::String, i)
    }

    fn path(&mut self, p: &str) -> u32 {
        if let Some(&i) = self.path_index.get(p) {
            return i;
        }
        let i = self.paths.len() as u32;
        self.paths.push(p.to_string());
        self.path_index.insert(p.to_string(), i);
        i
    }

    fn put(&mut self, data: &[u8]) -> Result<()> {
        self.out.write_all(data)?;
        self.pos += data.len() as u64;
        Ok(())
    }

    fn align8(&mut self) -> Result<()> {
        let pad = (8 - self.pos % 8) % 8;
        self.put(&[0u8; 8][..pad as usize])
    }

    /// Plain little-endian values, buffered.
    fn put_all<T: Copy, const N: usize>(&mut self, xs: &[T], bytes: impl Fn(T) -> [u8; N]) -> Result<()> {
        let mut buf = Vec::with_capacity(N * xs.len().min(1 << 16));
        for chunk in xs.chunks(1 << 16) {
            buf.clear();
            for &x in chunk {
                buf.extend_from_slice(&bytes(x));
            }
            self.put(&buf)?;
        }
        Ok(())
    }

    fn plain_array<T: Copy, const N: usize>(&mut self, t: Ty, xs: &[T], bytes: impl Fn(T) -> [u8; N]) -> Result<Rep> {
        if xs.is_empty() {
            return Ok(Rep::array(t, 0, false));
        }
        self.align8()?;
        let rep = Rep::array(t, self.pos, false);
        self.put(&(xs.len() as u64).to_le_bytes())?;
        self.put_all(xs, bytes)?;
        Ok(rep)
    }

    fn int_array(&mut self, xs: &[u32]) -> Result<Rep> {
        if xs.len() < MIN_COMPRESSED {
            return self.plain_array(Ty::Int, xs, u32::to_le_bytes);
        }
        let rep = Rep::array(Ty::Int, self.pos, true);
        self.put(&(xs.len() as u64).to_le_bytes())?;
        let mut body = Vec::new();
        compress_ints(xs, &mut body, self)?;
        Ok(rep)
    }

    /// Write a value out of line, `bytes` as they are.
    fn value_at(&mut self, t: Ty, bytes: &[u8]) -> Result<Rep> {
        let rep = Rep::at(t, self.pos);
        self.put(bytes)?;
        Ok(rep)
    }

    fn token_vector(&mut self, ts: &[u32]) -> Result<Rep> {
        let mut b = (ts.len() as u64).to_le_bytes().to_vec();
        ts.iter().for_each(|t| b.extend_from_slice(&t.to_le_bytes()));
        self.value_at(Ty::TokenVector, &b)
    }

    /// A list op with only explicit (`bits` 0x03) or prepended (0x20) items.
    fn list_op(&mut self, t: Ty, bits: u8, items: &[u32]) -> Result<Rep> {
        let mut b = vec![bits];
        b.extend_from_slice(&(items.len() as u64).to_le_bytes());
        items.iter().for_each(|i| b.extend_from_slice(&i.to_le_bytes()));
        self.value_at(t, &b)
    }

    fn value(&mut self, v: &Val) -> Result<Option<Rep>> {
        let f32_of = |n: &Num| n.value() as f32;
        Ok(Some(match v {
            Val::None => return Ok(None),
            Val::Token(s) => self.token_rep(s),
            Val::Str(s) => self.string_rep(s),
            Val::Bool(b) => Rep::inline(Ty::Bool, *b as u32),
            Val::Float(n) => Rep::inline(Ty::Float, f32_of(n).to_bits()),
            Val::Color(c) => {
                let b: Vec<u8> = c.iter().flat_map(|n| f32_of(n).to_le_bytes()).collect();
                self.value_at(Ty::Vec3f, &b)?
            }
            Val::Matrix(m) => {
                let b: Vec<u8> = m.iter().flatten().flat_map(|x| x.to_le_bytes()).collect();
                self.value_at(Ty::Matrix4d, &b)?
            }
            Val::Ints(xs) => self.int_array(xs)?,
            Val::Floats(xs, _) => self.plain_array(Ty::Float, xs, f32::to_le_bytes)?,
            Val::Vec3s(xs, _) => self.plain_array(Ty::Vec3f, xs, |p: [f32; 3]| {
                let mut b = [0u8; 12];
                (0..3).for_each(|k| b[4 * k..4 * k + 4].copy_from_slice(&p[k].to_le_bytes()));
                b
            })?,
            Val::Vec2s(xs, _) => self.plain_array(Ty::Vec2f, xs, |p: [f32; 2]| {
                let mut b = [0u8; 8];
                (0..2).for_each(|k| b[4 * k..4 * k + 4].copy_from_slice(&p[k].to_le_bytes()));
                b
            })?,
            // GfQuath is stored imaginary part first, the real part last.
            Val::Quats(xs, _) => self.plain_array(Ty::Quath, xs, |q: [f32; 4]| {
                let mut b = [0u8; 8];
                for (k, &c) in [q[1], q[2], q[3], q[0]].iter().enumerate() {
                    b[2 * k..2 * k + 2].copy_from_slice(&f16_bits(c).to_le_bytes());
                }
                b
            })?,
            Val::Tokens(ts) => {
                let idx: Vec<u32> = ts.iter().map(|t| self.token(t)).collect();
                self.plain_array(Ty::Token, &idx, u32::to_le_bytes)?
            }
        }))
    }

    fn field(&mut self, (token, rep): (u32, Rep)) -> u32 {
        let next = self.fields.len() as u32;
        let i = *self.field_index.entry((token, rep)).or_insert(next);
        if i == next {
            self.fields.push((token, rep));
        }
        i
    }

    fn spec(&mut self, path: &str, kind: u32, fields: Vec<(u32, Rep)>) {
        let set = self.field_sets.len() as u32;
        for f in fields {
            let i = self.field(f);
            self.field_sets.push(i);
        }
        self.field_sets.push(u32::MAX);
        let p = self.path(path);
        self.specs.push((p, set, kind));
    }

    fn property_path(&self, name: &str) -> String {
        format!("{}.{name}", self.stack.last().map_or("/", |p| p.path.as_str()))
    }

    fn add_property(&mut self, name: &str) {
        let t = self.token(name);
        if let Some(p) = self.stack.last_mut() {
            p.properties.push(t);
        }
    }

    /// Write the structure, the table of contents and the header. Returns
    /// the layer's size.
    pub(super) fn finish(mut self) -> Result<u64> {
        if self.stack.len() != 1 {
            bail!("usdc: {} prims left open", self.stack.len() - 1);
        }
        self.close_prim(SPEC_PSEUDO_ROOT)?;
        // Before the tokens go out: the paths add their elements to them.
        let (indexes, elements, jumps) = self.path_tree();
        let mut toc: Vec<(&str, u64, u64)> = Vec::new();

        let start = self.pos;
        let mut data = Vec::new();
        for t in &self.tokens {
            data.extend_from_slice(t.as_bytes());
            data.push(0);
        }
        self.put(&(self.tokens.len() as u64).to_le_bytes())?;
        self.put(&(data.len() as u64).to_le_bytes())?;
        self.put(&lz4_size(data.len() as u64).to_le_bytes())?;
        lz4_frame(&data, &mut self)?;
        toc.push(("TOKENS", start, self.pos - start));

        let start = self.pos;
        let strings = std::mem::take(&mut self.strings);
        self.put(&(strings.len() as u64).to_le_bytes())?;
        self.put_all(&strings, u32::to_le_bytes)?;
        toc.push(("STRINGS", start, self.pos - start));

        let start = self.pos;
        let fields = std::mem::take(&mut self.fields);
        self.put(&(fields.len() as u64).to_le_bytes())?;
        let tokens: Vec<u32> = fields.iter().map(|f| f.0).collect();
        compress_ints(&tokens, &mut Vec::new(), &mut self)?;
        let reps: Vec<u8> = fields.iter().flat_map(|f| f.1 .0.to_le_bytes()).collect();
        self.put(&lz4_size(reps.len() as u64).to_le_bytes())?;
        lz4_frame(&reps, &mut self)?;
        toc.push(("FIELDS", start, self.pos - start));

        let start = self.pos;
        let sets = std::mem::take(&mut self.field_sets);
        self.put(&(sets.len() as u64).to_le_bytes())?;
        compress_ints(&sets, &mut Vec::new(), &mut self)?;
        toc.push(("FIELDSETS", start, self.pos - start));

        let start = self.pos;
        self.put(&(self.paths.len() as u64).to_le_bytes())?;
        self.put(&(indexes.len() as u64).to_le_bytes())?;
        compress_ints(&indexes, &mut Vec::new(), &mut self)?;
        compress_ints(&elements, &mut Vec::new(), &mut self)?;
        compress_ints(&jumps, &mut Vec::new(), &mut self)?;
        toc.push(("PATHS", start, self.pos - start));

        let start = self.pos;
        let specs = std::mem::take(&mut self.specs);
        self.put(&(specs.len() as u64).to_le_bytes())?;
        for k in 0..3 {
            let col: Vec<u32> = specs.iter().map(|s| [s.0, s.1, s.2][k]).collect();
            compress_ints(&col, &mut Vec::new(), &mut self)?;
        }
        toc.push(("SPECS", start, self.pos - start));

        let toc_at = self.pos;
        let mut b = (toc.len() as u64).to_le_bytes().to_vec();
        for (name, start, size) in &toc {
            let mut n = [0u8; 16];
            n[..name.len()].copy_from_slice(name.as_bytes());
            b.extend_from_slice(&n);
            b.extend_from_slice(&start.to_le_bytes());
            b.extend_from_slice(&size.to_le_bytes());
        }
        self.put(&b)?;

        let mut header = [0u8; 88];
        header[..8].copy_from_slice(b"PXR-USDC");
        header[8..11].copy_from_slice(&VERSION);
        header[16..24].copy_from_slice(&toc_at.to_le_bytes());
        self.out.patch(0, &header)?;
        Ok(self.pos)
    }

    /// The paths as a tree, depth first (`_WriteCompressedPathData`): each
    /// path's index, its last element (negative for a property name) and
    /// where its next sibling is (-2 leaf, -1 child only, 0 sibling only,
    /// else the sibling's distance; a child always follows its parent).
    fn path_tree(&mut self) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        // Every path's parent is a path too (targets may name ones without
        // a spec).
        let mut parent_of: Vec<Option<(u32, i32)>> = Vec::new();
        let mut i = 0;
        while i < self.paths.len() {
            let p = self.paths[i].clone();
            let split = match p.rfind(['/', '.']) {
                _ if p == "/" => None,
                Some(k) if p.as_bytes()[k] == b'.' => Some((p[..k].to_string(), &p[k + 1..], true)),
                Some(0) => Some(("/".to_string(), &p[1..], false)),
                Some(k) => Some((p[..k].to_string(), &p[k + 1..], false)),
                None => None,
            };
            parent_of.push(split.map(|(parent, elem, prop)| {
                let t = self.token(elem) as i32;
                (self.path(&parent), if prop { -t } else { t })
            }));
            i += 1;
        }
        let n = self.paths.len();
        let mut children: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut element = vec![0i32; n];
        let mut root = None;
        for (i, p) in parent_of.iter().enumerate() {
            match p {
                Some((parent, e)) => {
                    children[*parent as usize].push(i as u32);
                    element[i] = *e;
                }
                None => root = Some(i as u32),
            }
        }
        let mut out = (Vec::new(), Vec::new(), Vec::new());
        // Depth first: (node, whether a sibling follows it).
        fn walk(node: u32, has_sibling: bool, children: &[Vec<u32>], element: &[i32], out: &mut (Vec<u32>, Vec<u32>, Vec<u32>)) {
            let me = out.0.len();
            out.0.push(node);
            out.1.push(element[node as usize] as u32);
            out.2.push(0);
            let kids = &children[node as usize];
            for (k, &c) in kids.iter().enumerate() {
                walk(c, k + 1 < kids.len(), children, element, out);
            }
            let jump: i32 = match (kids.is_empty(), has_sibling) {
                (false, true) => (out.0.len() - me) as i32,
                (true, true) => 0,
                (false, false) => -1,
                (true, false) => -2,
            };
            out.2[me] = jump as u32;
        }
        if let Some(r) = root {
            walk(r, false, &children, &element, &mut out);
        }
        out
    }

    fn close_prim(&mut self, kind: u32) -> Result<()> {
        let Some(mut p) = self.stack.pop() else { bail!("usdc: no prim open") };
        if !p.children.is_empty() {
            let f = (self.token("primChildren"), self.token_vector(&p.children)?);
            p.fields.push(f);
        }
        if !p.properties.is_empty() {
            let f = (self.token("properties"), self.token_vector(&p.properties)?);
            p.fields.push(f);
        }
        self.spec(&p.path, kind, p.fields);
        Ok(())
    }
}

impl<W: Patch> LayerSink for UsdcWriter<'_, W> {
    fn begin_prim(&mut self, ty: &str, name: &str, meta: &PrimMeta) -> Result<()> {
        let parent = self.stack.last().map_or("/".to_string(), |p| p.path.clone());
        let path = if parent == "/" { format!("/{name}") } else { format!("{parent}/{name}") };
        let t = self.token(name);
        if let Some(p) = self.stack.last_mut() {
            p.children.push(t);
        }
        let mut fields = vec![(self.token("specifier"), Rep::inline(Ty::Specifier, 0)), (self.token("typeName"), self.token_rep(ty))];
        if let Some(d) = meta.display_name {
            fields.push((self.token("displayName"), self.string_rep(d)));
        }
        if let Some(k) = meta.kind {
            fields.push((self.token("kind"), self.token_rep(k)));
        }
        if !meta.api_schemas.is_empty() {
            let items: Vec<u32> = meta.api_schemas.iter().map(|s| self.token(s)).collect();
            fields.push((self.token("apiSchemas"), self.list_op(Ty::TokenListOp, 0x20, &items)?));
        }
        self.stack.push(OpenPrim { path, fields, children: Vec::new(), properties: Vec::new() });
        Ok(())
    }

    fn end_prim(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            bail!("usdc: end_prim without a prim");
        }
        self.close_prim(SPEC_PRIM)
    }

    fn attr(&mut self, a: Attr) -> Result<()> {
        let path = self.property_path(a.name);
        self.add_property(a.name);
        let mut fields = vec![
            (self.token("custom"), Rep::inline(Ty::Bool, a.custom as u32)),
            (self.token("typeName"), self.token_rep(a.ty)),
            (self.token("variability"), Rep::inline(Ty::Variability, a.uniform as u32)),
        ];
        if let Some(rep) = self.value(&a.value)? {
            fields.push((self.token("default"), rep));
        }
        if let Some(i) = a.interpolation {
            fields.push((self.token("interpolation"), self.token_rep(i)));
        }
        if let Some(target) = a.connect {
            let p = self.path(target);
            fields.push((self.token("connectionPaths"), self.list_op(Ty::PathListOp, 0x03, &[p])?));
        }
        self.spec(&path, SPEC_ATTRIBUTE, fields);
        Ok(())
    }

    fn rel(&mut self, name: &str, targets: &[&str], _list: bool) -> Result<()> {
        let path = self.property_path(name);
        self.add_property(name);
        let items: Vec<u32> = targets.iter().map(|t| self.path(t)).collect();
        let fields = vec![(self.token("variability"), Rep::inline(Ty::Variability, 1)), (self.token("targetPaths"), self.list_op(Ty::PathListOp, 0x03, &items)?)];
        self.spec(&path, SPEC_RELATIONSHIP, fields);
        Ok(())
    }
}

/// The size of `n` bytes in TfFastCompression's framing of LZ4 blocks that
/// hold them as literals only (no matches — valid LZ4, just not smaller).
fn lz4_size(n: u64) -> u64 {
    let block = |m: u64| 1 + if m >= 15 { (m - 15) / 255 + 1 } else { 0 } + m;
    if n <= LZ4_MAX_INPUT {
        1 + block(n)
    } else {
        let (whole, part) = (n / LZ4_MAX_INPUT, n % LZ4_MAX_INPUT);
        1 + whole * (4 + block(LZ4_MAX_INPUT)) + if part > 0 { 4 + block(part) } else { 0 }
    }
}

/// An LZ4 block's header for `m` literal bytes: the token and its length
/// extension.
fn lz4_block_header(m: u64) -> Vec<u8> {
    if m < 15 {
        return vec![(m as u8) << 4];
    }
    let mut h = vec![0xF0];
    let mut rest = m - 15;
    while rest >= 255 {
        h.push(255);
        rest -= 255;
    }
    h.push(rest as u8);
    h
}

/// `data` framed as `lz4_size` counts it.
fn lz4_frame<W: Patch>(data: &[u8], w: &mut UsdcWriter<W>) -> Result<()> {
    let mut f = Lz4Frame::new(data.len() as u64, w)?;
    f.write(data, w)
}

/// Streams bytes into TfFastCompression's framing (`lz4_size`), emitting
/// each chunk's header as its first byte comes.
struct Lz4Frame {
    total: u64,
    done: u64,
}

impl Lz4Frame {
    fn new<W: Patch>(total: u64, w: &mut UsdcWriter<W>) -> Result<Lz4Frame> {
        let chunks = if total <= LZ4_MAX_INPUT { 0 } else { total.div_ceil(LZ4_MAX_INPUT) };
        w.put(&[chunks as u8])?;
        if chunks == 0 {
            w.put(&lz4_block_header(total))?;
        }
        Ok(Lz4Frame { total, done: 0 })
    }

    fn write<W: Patch>(&mut self, mut data: &[u8], w: &mut UsdcWriter<W>) -> Result<()> {
        let chunked = self.total > LZ4_MAX_INPUT;
        while !data.is_empty() {
            let into_chunk = self.done % LZ4_MAX_INPUT;
            if chunked && into_chunk == 0 {
                let m = (self.total - self.done).min(LZ4_MAX_INPUT);
                let h = lz4_block_header(m);
                w.put(&((h.len() as u64 + m) as u32).to_le_bytes())?;
                w.put(&h)?;
            }
            let take = ((LZ4_MAX_INPUT - into_chunk) as usize).min(data.len());
            w.put(&data[..take])?;
            self.done += take as u64;
            data = &data[take..];
        }
        Ok(())
    }
}

/// Pixar's integer compression (`Sdf_IntegerCompression`), written as it is
/// computed: the size, then the encoding — the most common difference
/// between neighbours, 2-bit codes (common / 8 / 16 / 32-bit difference),
/// the differences that aren't the common one — framed as LZ4.
fn compress_ints<W: Patch>(xs: &[u32], buf: &mut Vec<u8>, w: &mut UsdcWriter<W>) -> Result<()> {
    let delta = |k: usize| (xs[k] as i32).wrapping_sub(if k == 0 { 0 } else { xs[k - 1] as i32 });
    // The common difference among the small ones (any value decodes).
    let mut counts = [0usize; 256];
    for k in 0..xs.len() {
        let d = delta(k);
        if (-128..128).contains(&d) {
            counts[(d + 128) as usize] += 1;
        }
    }
    let common = counts.iter().enumerate().max_by_key(|&(i, &c)| (c, i)).map_or(0, |(i, _)| i as i32 - 128);
    let code = |d: i32| -> u8 {
        if d == common {
            0
        } else if i8::try_from(d).is_ok() {
            1
        } else if i16::try_from(d).is_ok() {
            2
        } else {
            3
        }
    };
    let codes_len = (xs.len() * 2).div_ceil(8);
    let vints: usize = (0..xs.len()).map(|k| [0, 1, 2, 4][code(delta(k)) as usize]).sum();
    let encoded = (4 + codes_len + vints) as u64;
    w.put(&lz4_size(encoded).to_le_bytes())?;
    let mut f = Lz4Frame::new(encoded, w)?;
    f.write(&common.to_le_bytes(), w)?;
    const BLOCK: usize = 1 << 16;
    for start in (0..xs.len()).step_by(4 * BLOCK) {
        buf.clear();
        let end = (start + 4 * BLOCK).min(xs.len());
        for group in (start..end).step_by(4) {
            let mut byte = 0u8;
            for k in group..(group + 4).min(end) {
                byte |= code(delta(k)) << (2 * (k - group));
            }
            buf.push(byte);
        }
        f.write(buf, w)?;
    }
    for start in (0..xs.len()).step_by(BLOCK) {
        buf.clear();
        for k in start..(start + BLOCK).min(xs.len()) {
            let d = delta(k);
            match code(d) {
                0 => {}
                1 => buf.push(d as i8 as u8),
                2 => buf.extend_from_slice(&(d as i16).to_le_bytes()),
                _ => buf.extend_from_slice(&d.to_le_bytes()),
            }
        }
        f.write(buf, w)?;
    }
    Ok(())
}

/// IEEE half-precision bits of `x`, rounded to nearest.
fn f16_bits(x: f32) -> u16 {
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32;
    let mant = b & 0x7f_ffff;
    if exp == 0xff {
        return sign | 0x7c00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = mant | 0x80_0000;
        let shift = (14 - e) as u32;
        let half = 1u32 << (shift - 1);
        let r = (m + half - 1 + ((m >> shift) & 1)) >> shift;
        return sign | r as u16;
    }
    let r = mant + 0xfff + ((mant >> 13) & 1);
    let (e, r) = if r & 0x80_0000 != 0 { (e + 1, 0) } else { (e, r) };
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    sign | ((e as u16) << 10) | (r >> 13) as u16
}

#[cfg(test)]
pub(super) fn f16_bits_for_test(x: f32) -> u16 {
    f16_bits(x)
}
