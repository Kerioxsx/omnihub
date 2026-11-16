# Vesper Client 0.3 models, built in Blender (run headless):
#   blender -b --python client/models/vesper_extras.py -- <out_dir> [render_dir]
# Writes horns.obj (the Wraith head's horns) and three cars for the car emote:
# car_super.obj, car_muscle.obj, car_hyper.obj. Units are blocks; OBJ +Y up,
# +Z toward the front. Original, unbranded designs generated from code: no
# imported art, no real car's shape, name or logo.

import bpy, bmesh, math, sys, os
from mathutils import Vector

argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
OUT = argv[0] if argv else os.path.join(os.path.dirname(__file__), "..", "src", "main", "resources", "assets", "vesper", "models")
RENDER = argv[1] if len(argv) > 1 else None
os.makedirs(OUT, exist_ok=True)


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def mesh_obj(name, bm):
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    return ob


def tube(bm, a, b, r0, r1, sides=8):
    a, b = Vector(a), Vector(b)
    d = (b - a).normalized()
    up = Vector((0, 0, 1)) if abs(d.z) < 0.9 else Vector((1, 0, 0))
    u = d.cross(up).normalized()
    v = d.cross(u).normalized()
    ra = [bm.verts.new(a + (u * math.cos(2 * math.pi * i / sides) + v * math.sin(2 * math.pi * i / sides)) * r0) for i in range(sides)]
    rb = [bm.verts.new(b + (u * math.cos(2 * math.pi * i / sides) + v * math.sin(2 * math.pi * i / sides)) * r1) for i in range(sides)]
    for i in range(sides):
        j = (i + 1) % sides
        bm.faces.new((ra[i], ra[j], rb[j], rb[i]))
    bm.faces.new(list(reversed(ra)))
    bm.faces.new(rb)


def spike(bm, base, tip, r, sides=5):
    base, tip = Vector(base), Vector(tip)
    d = (tip - base).normalized()
    up = Vector((0, 0, 1)) if abs(d.z) < 0.9 else Vector((1, 0, 0))
    u = d.cross(up).normalized()
    v = d.cross(u).normalized()
    ring = [bm.verts.new(base + (u * math.cos(2 * math.pi * i / sides) + v * math.sin(2 * math.pi * i / sides)) * r) for i in range(sides)]
    t = bm.verts.new(tip)
    for i in range(sides):
        bm.faces.new((ring[i], ring[(i + 1) % sides], t))
    bm.faces.new(list(reversed(ring)))


def box(bm, c, s):
    cx, cy, cz = c
    sx, sy, sz = s
    v = [bm.verts.new((cx + dx * sx / 2, cy + dy * sy / 2, cz + dz * sz / 2)) for dx in (-1, 1) for dy in (-1, 1) for dz in (-1, 1)]
    for f in ((0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)):
        bm.faces.new([v[i] for i in f])


def export(name):
    for ob in bpy.context.scene.objects:
        ob.select_set(ob.type == "MESH")
    bpy.ops.wm.obj_export(filepath=os.path.join(OUT, name), export_selected_objects=True, export_materials=False,
                          export_uv=False, export_normals=False, export_triangulated_mesh=True,
                          forward_axis="Z", up_axis="Y")
    print("wrote", name)


# ---------------------------------------------------------------- horns

def horn_path(side):
    """Rooted at the top corner of the head (z = 0 is the root height): out
    to the side, sweeping up the outside, the tip curling back in over the
    flame. A cubic curve, so it never dips below the root."""
    p0, p1, p2, p3 = (0.24, 0.0), (0.62, 0.03), (0.80, 0.56), (0.46, 0.98)
    pts = []
    n = 26
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        x = u ** 3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t ** 3 * p3[0]
        z = u ** 3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t ** 3 * p3[1]
        y = -0.07 * math.sin(t * math.pi)
        pts.append(Vector((side * x, y, z)))
    return pts


def build_horns():
    reset()
    parts = {k: bmesh.new() for k in ("horn_base", "horn_mid", "horn_tip", "bands", "spikes", "spikes_tip")}
    for side in (-1, 1):
        p = horn_path(side)
        n = len(p) - 1
        for i in range(n):
            t0, t1 = i / n, (i + 1) / n
            r0, r1 = 0.085 * (1 - t0) ** 0.85 + 0.006, 0.085 * (1 - t1) ** 0.85 + 0.006
            g = "horn_base" if t1 <= 0.5 else "horn_mid" if t1 <= 0.75 else "horn_tip"
            tube(parts[g], p[i], p[i + 1], r0, r1, 10)
        # Dark bands around the lower horn.
        for i in (3, 6, 9, 12):
            d = (p[i + 1] - p[i - 1]).normalized()
            r = 0.085 * (1 - i / n) ** 0.85 + 0.018
            tube(parts["bands"], p[i] - d * 0.018, p[i] + d * 0.018, r, r, 12)
        # Thorns along the outer edge (away from the flame) and a few behind.
        for k, i in enumerate((5, 8, 10, 12, 14, 16, 18, 20, 22)):
            a = p[i]
            d = (p[i + 1] - p[i - 1]).normalized()
            centre = Vector((side * 0.5, 0, 0.5))
            out = (a - centre)
            out.y = 0
            out = (out - d * out.dot(d)).normalized()
            if k % 3 == 2:
                out = (out * 0.4 + Vector((0, -1, 0)) * 0.9).normalized()
            length = 0.11 + 0.05 * (1 - abs(i - 13) / 13)
            g = "spikes_tip" if i >= 17 else "spikes"
            spike(parts[g], a, a + out * length + d * 0.05, 0.02 + 0.008 * (1 - i / n))
    for k, bm in parts.items():
        mesh_obj(k, bm)
    export("horns.obj")


# ---------------------------------------------------------------- cars

def bezier(points, steps=10):
    """Smooth a polyline with a quadratic B-spline (keeps the first and last point)."""
    out = [points[0]]
    for i in range(1, len(points) - 1):
        p0 = [(points[i - 1][k] + points[i][k]) / 2 for k in range(2)]
        p2 = [(points[i][k] + points[i + 1][k]) / 2 for k in range(2)]
        for s in range(steps):
            t = s / steps
            out.append(tuple((1 - t) ** 2 * p0[k] + 2 * (1 - t) * t * points[i][k] + t * t * p2[k] for k in range(2)))
    out.append(points[-1])
    return out


def extrude_profile(bm, prof, width, taper=0.0):
    """A side profile (list of (y, z)) extruded across x; `taper` narrows the top."""
    zmin = min(z for _, z in prof)
    zmax = max(z for _, z in prof)
    def half(z):
        return width / 2 * (1 - taper * (z - zmin) / max(1e-6, zmax - zmin))
    left = [bm.verts.new((-half(z), y, z)) for y, z in prof]
    right = [bm.verts.new((half(z), y, z)) for y, z in prof]
    n = len(prof)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((left[i], left[j], right[j], right[i]))
    bm.faces.new(list(reversed(left)))
    bm.faces.new(right)


def wheel(tyre, rim, cal, x, y, r, w, side):
    sides = 24
    def ring(bm, rad, xx):
        return [bm.verts.new((xx, y + rad * math.cos(2 * math.pi * i / sides), r + rad * math.sin(2 * math.pi * i / sides))) for i in range(sides)]
    a, b = ring(tyre, r, x - w / 2), ring(tyre, r, x + w / 2)
    for i in range(sides):
        j = (i + 1) % sides
        tyre.faces.new((a[i], a[j], b[j], b[i]))
    tyre.faces.new(list(reversed(a)))
    tyre.faces.new(b)
    # Rim: a slightly proud disc with five spokes on the outer face.
    xo = x + side * (w / 2 + 0.005)
    hub = rim.verts.new((xo, y, r))
    outer = ring(rim, r * 0.72, xo)
    for i in range(sides):
        if (i * 5 // sides) != ((i + 1) * 5 // sides) or i % 5 in (0, 1):
            rim.faces.new((hub, outer[i], outer[(i + 1) % sides]))
    lip_a, lip_b = ring(rim, r * 0.72, xo), ring(rim, r * 0.8, xo)
    for i in range(sides):
        j = (i + 1) % sides
        rim.faces.new((lip_a[i], lip_a[j], lip_b[j], lip_b[i]))
    # Brake calliper peeking through behind the spokes.
    box(cal, (x + side * (w / 2 - 0.04), y + r * 0.35, r + r * 0.25), (0.06, 0.16, 0.22))


CARS = {
    # control points of the side silhouette (y forward, z up), smoothed; cabin; width; wheel radius; axles
    "super": dict(
        body=[(-2.25, 0.32), (-1.2, 0.3), (1.2, 0.3), (2.3, 0.33), (2.38, 0.5), (1.9, 0.66), (1.0, 0.8), (-0.6, 0.98), (-1.9, 0.98), (-2.3, 0.86)],
        cabin=[(-1.05, 0.95), (-0.2, 0.96), (1.0, 0.8), (0.55, 1.08), (0.0, 1.22), (-0.75, 1.2)],
        width=2.02, r=0.37, axles=(1.42, -1.45), wing=True, scoop=True),
    "muscle": dict(
        body=[(-2.35, 0.36), (2.38, 0.36), (2.42, 0.78), (2.2, 0.98), (0.6, 1.02), (-1.5, 1.02), (-2.3, 1.0)],
        cabin=[(-1.35, 1.0), (0.55, 1.0), (0.1, 1.5), (-1.05, 1.52)],
        width=2.0, r=0.41, axles=(1.5, -1.5), wing=False, scoop=False),
    "hyper": dict(
        body=[(-2.3, 0.28), (-1.2, 0.26), (1.3, 0.26), (2.42, 0.28), (2.46, 0.42), (1.8, 0.58), (0.9, 0.7), (-0.8, 0.9), (-1.8, 0.88), (-2.3, 0.78)],
        cabin=[(-0.95, 0.86), (-0.1, 0.86), (1.25, 0.68), (0.6, 1.0), (0.05, 1.12), (-0.6, 1.1)],
        width=2.06, r=0.35, axles=(1.55, -1.5), wing=True, scoop=True),
}


def build_car(kind):
    reset()
    c = CARS[kind]
    W = c["width"]
    bm = bmesh.new()
    extrude_profile(bm, bezier(c["body"], 8), W, taper=0.12)
    mesh_obj("body", bm)
    sk = bmesh.new()
    box(sk, (0, 0, 0.36), (W + 0.06, 3.0, 0.12))          # side skirts
    box(sk, (0, c["body"][3][0] - 0.05, 0.3), (W * 0.92, 0.12, 0.08))  # front splitter
    if c["wing"]:
        box(sk, (0, -2.05, 1.18), (W * 0.96, 0.4, 0.05))
        for x in (-0.55, 0.55):
            box(sk, (x, -1.98, 1.02), (0.05, 0.16, 0.3))
        for x in (-W * 0.48, W * 0.48):
            box(sk, (x, -2.05, 1.18), (0.04, 0.42, 0.14))  # wing end plates
    if c["scoop"]:
        for x in (-W / 2 - 0.01, W / 2 + 0.01):
            box(sk, (x, -0.65, 0.62), (0.03, 0.75, 0.22))  # side air intakes
    if kind == "muscle":
        box(sk, (0, 1.25, 1.06), (0.62, 1.1, 0.1))         # hood scoop
    mesh_obj("trim", sk)
    g = bmesh.new()
    extrude_profile(g, bezier(c["cabin"], 6), W * 0.8, taper=0.18)
    mesh_obj("glass", g)
    tyre, rim, cal = bmesh.new(), bmesh.new(), bmesh.new()
    for y in c["axles"]:
        for side in (-1, 1):
            wheel(tyre, rim, cal, side * (W / 2 - 0.13), y, c["r"], 0.32, side)
    mesh_obj("wheels", tyre)
    mesh_obj("rims", rim)
    mesh_obj("calipers", cal)
    l = bmesh.new()
    front = max(p[0] for p in c["body"])
    back = min(p[0] for p in c["body"])
    for x in (-W / 2 + 0.32, W / 2 - 0.32):
        box(l, (x, front - 0.06, 0.52), (0.46, 0.05, 0.05))   # thin LED strips
    mesh_obj("lights", l)
    t = bmesh.new()
    box(t, (0, back + 0.01, 0.74 if kind != "hyper" else 0.64), (W * 0.9, 0.04, 0.05))
    mesh_obj("tail", t)
    export("car_%s.obj" % kind)


build_horns()
for k in CARS:
    build_car(k)
