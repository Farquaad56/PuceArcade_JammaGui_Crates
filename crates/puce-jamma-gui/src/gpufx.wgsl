// Filtres vidéo sur carte graphique : même calcul que postfx.rs / crt.rs / crt_light.rs, pixel
// par pixel, en entiers là où le processeur calcule en entiers. La structure P (paramètres) est
// ajoutée devant ce fichier par gpufx_data::uniform_wgsl.
//
// Passes : fs_beam (scanlines, interpolation, gamma) -> fs_tube (courbure, instabilité,
// convergence, canaux, balayage) -> fs_down + fs_blur_h/v (halo) -> fs_final (halo, couleur,
// masque, vignettage, coins).

@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var tab_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_a: texture_2d<f32>;
@group(0) @binding(3) var tex_b: texture_2d<f32>;

struct VO {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Triangle couvrant toute la zone de rendu ; uv = 0..1 sur la zone.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VO {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: VO;
    o.pos = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

// ---------- accès ----------

fn tab(i: u32) -> f32 {
    return textureLoad(tab_tex, vec2<i32>(i32(i % 1024u), i32(i / 1024u)), 0).r;
}

fn tabi(i: u32) -> i32 {
    return i32(tab(i));
}

fn tabu(i: u32) -> u32 {
    return u32(tab(i));
}

// Pixel 8 bits de tex_a, en entiers 0..255.
fn px_a(x: i32, y: i32) -> vec3<i32> {
    let c = textureLoad(tex_a, vec2<i32>(x, y), 0);
    return vec3<i32>(round(c.rgb * 255.0));
}

fn out8(v: vec3<u32>) -> vec4<f32> {
    return vec4<f32>(vec3<f32>(v) / 255.0, 1.0);
}

// ---------- passe 1 : faisceau ----------

// Faisceau gaussien : colonne source cx de la rangée de sortie y.
fn vert_gauss(cx: i32, y: u32) -> vec3<i32> {
    let sh = i32(p.sh);
    let pf = (f32(y) + 0.5) * f32(p.sh) / f32(p.h);
    let r0 = i32(floor(pf - 0.5));
    var a = vec3<f32>(0.0);
    for (var j = -1; j <= 2; j++) {
        let r = r0 + j;
        if r < 0 || r >= sh {
            continue;
        }
        let d = abs(pf - (f32(r) + 0.5));
        let di = min(u32(d * 32.0), 64u);
        let v = px_a(cx, r);
        for (var c = 0; c < 3; c++) {
            let w = tab(p.o_beam + u32(v[c] >> 3u) * 65u + di);
            a[c] += w * f32(v[c]);
        }
    }
    return vec3<i32>(min(a + 0.5, vec3<f32>(255.0)));
}

// Filtre vertical (scanlines) : colonne source cx de la rangée de sortie y.
fn vert(cx: i32, y: u32) -> vec3<i32> {
    let sh = i32(p.sh);
    if p.mode == 0u {
        let r = min((y * p.sh) / p.h, p.sh - 1u);
        return px_a(cx, i32(r));
    }
    if p.mode == 3u {
        return vert_gauss(cx, y);
    }
    let pos = f32(y) * f32(p.sh) / f32(p.h);
    let fl = floor(pos);
    let frac = pos - fl;
    let i0 = i32(fl);
    var rows: array<vec3<i32>, 4>;
    for (var j = 0; j < 4; j++) {
        rows[j] = px_a(cx, clamp(i0 - 1 + j, 0, sh - 1));
    }
    if p.mode == 1u {
        let ph = min(u32(frac * 64.0), 63u);
        var a = vec3<i32>(64);
        for (var j = 0u; j < 4u; j++) {
            a += tabi(p.o_std + ph * 4u + j) * rows[j];
        }
        return clamp(a >> vec3<u32>(7u), vec3<i32>(0), vec3<i32>(255));
    }
    // adaptatives
    let ph = min(u32(frac * 256.0), 255u);
    var near = 2;
    if frac <= 0.5 {
        near = 1;
    }
    let n = vec3<f32>(rows[near]);
    let l = (0.299 * n.x + 0.587 * n.y + 0.114 * n.z) / 255.0;
    var a = vec3<f32>(0.5);
    for (var j = 0u; j < 4u; j++) {
        let d = tab(p.o_dark + ph * 4u + j);
        let b = tab(p.o_bright + ph * 4u + j);
        let wg = (d + (b - d) * l) / 256.0;
        a += wg * vec3<f32>(rows[j]);
    }
    return vec3<i32>(clamp(a, vec3<f32>(0.0), vec3<f32>(255.0)));
}

@fragment
fn fs_beam(i: VO) -> @location(0) vec4<f32> {
    let x = u32(i.pos.x);
    let y = u32(i.pos.y);
    let sw = i32(p.sw);
    var v: vec3<i32>;
    if p.kernel_on != 0u {
        let pos = (f32(x) + 0.5) * f32(p.sw) / f32(p.w) - 0.5;
        let fi = floor(pos);
        let ph = min(u32((pos - fi) * 64.0), 63u);
        var a = vec3<i32>(64);
        for (var j = 0; j < 4; j++) {
            let idx = clamp(i32(fi) - 1 + j, 0, sw - 1);
            a += tabi(p.o_kern + ph * 4u + u32(j)) * vert(idx, y);
        }
        v = clamp(a >> vec3<u32>(7u), vec3<i32>(0), vec3<i32>(255));
    } else {
        v = vert(i32(min(x * p.sw / p.w, p.sw - 1u)), y);
    }
    let g = vec3<u32>(tabu(p.o_gamma + u32(v.x)), tabu(p.o_gamma + u32(v.y)), tabu(p.o_gamma + u32(v.z)));
    return out8(g);
}

// ---------- passe 2 : géométrie et tube ----------

// Image du faisceau (tex_a), noir hors de l'image.
fn t1(x: i32, y: i32) -> vec3<u32> {
    if x < 0 || y < 0 || x >= i32(p.w) || y >= i32(p.h) {
        return vec3<u32>(0u);
    }
    return vec3<u32>(px_a(x, y));
}

// Échantillon bilinéaire en virgule fixe (poids sur 8 bits), comme crt::bilinear.
fn bilinear(sx: f32, sy: f32) -> vec3<u32> {
    let xi = i32(sx + 1.0) - 1;
    let yi = i32(sy + 1.0) - 1;
    let ax = u32((sx - f32(xi)) * 256.0);
    let ay = u32((sy - f32(yi)) * 256.0);
    let top = t1(xi, yi) * (256u - ax) + t1(xi + 1, yi) * ax;
    let bot = t1(xi, yi + 1) * (256u - ax) + t1(xi + 1, yi + 1) * ax;
    return (top * (256u - ay) + bot * ay + vec3<u32>(32768u)) >> vec3<u32>(16u);
}

// Image après courbure et instabilité, au pixel (x, y) (dans l'image).
fn warped(x: i32, y: i32) -> vec3<u32> {
    if p.warp_on == 0u {
        return t1(x, y);
    }
    let wf = f32(p.w);
    let hf = f32(p.h);
    let dx = tab(p.o_jitter + u32(y));
    let v = (f32(y) + 0.5) / hf * 2.0 - 1.0;
    let ku = 1.0 + v * v * p.k0;
    let a = ku * 0.5 * wf;
    let b = 0.5 * wf - 0.5 - dx;
    let cc = v * 0.5 * hf;
    let d = 0.5 * hf - 0.5;
    let u = (f32(x) + 0.5) / wf * 2.0 - 1.0;
    let kv = 1.0 + u * u * p.k1;
    if abs(u) > 1.0 / ku || abs(v) * kv > 1.0 {
        return vec3<u32>(0u);
    }
    return bilinear(a * u + b, cc * kv + d);
}

// Canal décalé (convergence), noir hors de l'image.
fn shifted(x: i32, y: i32, dx: i32, dy: i32) -> vec3<u32> {
    let sx = x - dx;
    let sy = y - dy;
    if sx < 0 || sy < 0 || sx >= i32(p.w) || sy >= i32(p.h) {
        return vec3<u32>(0u);
    }
    return warped(sx, sy);
}

@fragment
fn fs_tube(i: VO) -> @location(0) vec4<f32> {
    let x = i32(i.pos.x);
    let y = i32(i.pos.y);
    let c = warped(x, y);
    if p.tube_on == 0u {
        return out8(c);
    }
    var rgb = c;
    if p.conv_on != 0u {
        rgb.x = shifted(x, y, p.conv_rx, p.conv_ry).x;
        rgb.z = shifted(x, y, p.conv_bx, p.conv_by).z;
    }
    let sg = tabu(p.o_sweep + u32(y));
    let lift = (max(sg, 256u) - 256u) * 40u / 256u;
    let cg = vec3<u32>(p.gr, p.gg, p.gb);
    let o = min(((rgb * cg * sg + vec3<u32>(32768u)) >> vec3<u32>(16u)) + lift, vec3<u32>(255u));
    return out8(o);
}

// ---------- halo ----------

// Réduction par blocs bs × bs, en lumière quasi linéaire (somme des carrés).
@fragment
fn fs_down(i: VO) -> @location(0) vec4<f32> {
    let bx = u32(i.pos.x);
    let by = u32(i.pos.y);
    var acc = vec3<f32>(0.0);
    let y1 = min((by + 1u) * p.bs, p.h);
    let x1 = min((bx + 1u) * p.bs, p.w);
    for (var y = by * p.bs; y < y1; y++) {
        for (var x = bx * p.bs; x < x1; x++) {
            let c = vec3<f32>(px_a(i32(x), i32(y))) / 255.0;
            acc += c * c;
        }
    }
    return vec4<f32>(acc, 1.0);
}

fn blur_at(x: i32, y: i32) -> vec3<f32> {
    return textureLoad(tex_b, vec2<i32>(x, y), 0).rgb;
}

// Flou en boîte de rayon br, bords répétés.
@fragment
fn fs_blur_h(i: VO) -> @location(0) vec4<f32> {
    let x = i32(i.pos.x);
    let y = i32(i.pos.y);
    let r = i32(p.br);
    var acc = vec3<f32>(0.0);
    for (var k = -r; k <= r; k++) {
        acc += blur_at(clamp(x + k, 0, i32(p.bw) - 1), y);
    }
    return vec4<f32>(acc * (1.0 / f32(2 * r + 1)), 1.0);
}

@fragment
fn fs_blur_v(i: VO) -> @location(0) vec4<f32> {
    let x = i32(i.pos.x);
    let y = i32(i.pos.y);
    let r = i32(p.br);
    var acc = vec3<f32>(0.0);
    for (var k = -r; k <= r; k++) {
        acc += blur_at(x, clamp(y + k, 0, i32(p.bh) - 1));
    }
    return vec4<f32>(acc * (1.0 / f32(2 * r + 1)), 1.0);
}

// Lueur 0..65280 d'un point de l'image réduite.
fn glow(x: u32, y: u32) -> vec3<u32> {
    let v = blur_at(i32(x), i32(y));
    return vec3<u32>(min(sqrt(max(v * p.glow_n, vec3<f32>(0.0))) * p.glow_k, vec3<f32>(65280.0)));
}

// Halo ajouté en mode « écran » au pixel (x, y), comme crt_light::bloom.
fn add_bloom(x: u32, y: u32, base: vec3<u32>) -> vec3<u32> {
    let s = f32(p.bs);
    let bxf = clamp((f32(x) + 0.5) / s - 0.5, 0.0, f32(p.bw - 1u));
    let byf = clamp((f32(y) + 0.5) / s - 0.5, 0.0, f32(p.bh - 1u));
    let x0 = u32(bxf);
    let x1 = min(x0 + 1u, p.bw - 1u);
    let y0 = u32(byf);
    let y1 = min(y0 + 1u, p.bh - 1u);
    let fx = u32(fract(bxf) * 256.0);
    let fy = u32(fract(byf) * 256.0);
    let la = (glow(x0, y0) * (256u - fy) + glow(x0, y1) * fy) >> vec3<u32>(8u);
    let lb = (glow(x1, y0) * (256u - fy) + glow(x1, y1) * fy) >> vec3<u32>(8u);
    let g = min((la * (256u - fx) + lb * fx) >> vec3<u32>(16u), vec3<u32>(255u));
    return min(base + ((g * (vec3<u32>(255u) - base) * 257u) >> vec3<u32>(16u)), vec3<u32>(255u));
}

// ---------- passe finale ----------

fn corner(x: u32, y: u32) -> f32 {
    let r = p.corner_r;
    let fx = f32(x) + 0.5;
    let fy = f32(y) + 0.5;
    let dx = max(r - min(fx, f32(p.w) - fx), 0.0);
    let dy = max(r - min(fy, f32(p.h) - fy), 0.0);
    if dx > 0.0 && dy > 0.0 {
        return clamp(0.5 - (sqrt(dx * dx + dy * dy) - r), 0.0, 1.0);
    }
    return 1.0;
}

fn mask_gain(x: u32, y: u32, c: u32) -> u32 {
    return tabu(p.o_mask + ((y % p.ph) * p.pw + x % p.pw) * 3u + c);
}

// Couleur, masque, vignettage et coins en entiers (sans taches de pureté).
fn finish_int(x: u32, y: u32, base: vec3<u32>) -> vec3<u32> {
    var vig = (tabu(p.o_vcol + x) * tabu(p.o_vrow + y) + 128u) >> 8u;
    let near_y = y < p.rc || y + p.rc >= p.h;
    if near_y && (x < p.rc || x + p.rc >= p.w) {
        vig = u32(f32(vig) * corner(x, y));
    }
    var o: vec3<u32>;
    for (var c = 0u; c < 3u; c++) {
        let l = tabu(p.o_lut + c * 256u + base[c]);
        o[c] = min((l * mask_gain(x, y, c) * vig + 32768u) >> 16u, 255u);
    }
    return o;
}

// Même passe avec taches de pureté, en flottants.
fn finish_float(x: u32, y: u32, base: vec3<u32>) -> vec3<u32> {
    let cx = vec3<f32>(tab(p.o_cols + x * 3u), tab(p.o_cols + x * 3u + 1u), tab(p.o_cols + x * 3u + 2u));
    let ry = vec3<f32>(tab(p.o_rows + y * 3u), tab(p.o_rows + y * 3u + 1u), tab(p.o_rows + y * 3u + 2u));
    var vig = cx.x * ry.x;
    let r = p.corner_r;
    if f32(y) < r || f32(y) + r >= f32(p.h) {
        vig *= corner(x, y);
    }
    let m = cx.y * ry.y;
    let g = cx.z * ry.z;
    let b = cx.z * ry.y * 0.7;
    let pg = vec3<f32>(1.0 + p.pk * (m - g - 0.5 * b), 1.0 + p.pk * (g - m - 0.5 * b), 1.0 + p.pk * (m + b - g));
    let t = vec3<f32>(p.t0, p.t1, p.t2);
    var o: vec3<u32>;
    for (var c = 0u; c < 3u; c++) {
        let v = min(p.black + f32(base[c]) * t[c] * pg[c] * p.lift, 255.0);
        o[c] = u32(min(v * f32(mask_gain(x, y, c)) * (1.0 / 256.0) * vig, 255.0));
    }
    return o;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_final(i: VO) -> @location(0) vec4<f32> {
    let x = min(u32(i.uv.x * f32(p.w)), p.w - 1u);
    let y = min(u32(i.uv.y * f32(p.h)), p.h - 1u);
    var c = vec3<u32>(px_a(i32(x), i32(y)));
    if p.bloom_on != 0u {
        c = add_bloom(x, y, c);
    }
    if p.fin_on != 0u {
        if p.fin_int != 0u {
            c = finish_int(x, y, c);
        } else {
            c = finish_float(x, y, c);
        }
    }
    var f = vec3<f32>(c) / 255.0;
    if p.srgb_out != 0u {
        f = srgb_to_linear(f);
    }
    return vec4<f32>(f, 1.0);
}
