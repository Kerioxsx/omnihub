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
    """Points from the side of the head, sweeping out, then curling up and in."""
    pts = []
    n = 16
    for i in range(n + 1):
        t = i / n
        # Out sideways first, then up; a slight backward lean.
        x = side * (0.27 + 0.42 * math.sin(min(1, t * 1.6) * math.pi / 2) - 0.08 * max(0, t - 0.7) / 0.3)
        z = 0.02 + 0.95 * (t ** 1.7)
        y = -0.06 * t
        pts.append(Vector((x, y, z)))
    return pts


def build_horns():
    reset()
    bm_base, bm_tip, bm_sp = bmesh.new(), bmesh.new(), bmesh.new()
    for side in (-1, 1):
        p = horn_path(side)
        n = len(p) - 1
        for i in range(n):
            t0, t1 = i / n, (i + 1) / n
            r0, r1 = 0.075 * (1 - t0) ** 0.9 + 0.008, 0.075 * (1 - t1) ** 0.9 + 0.008
            tube(bm_base if t1 <= 0.55 else bm_tip, p[i], p[i + 1], r0, r1)
        # Thorns along the blue part, pointing outward and back.
        for i in (9, 11, 13):
            a = p[i]
            d = (p[i + 1] - p[i - 1]).normalized()
            out = Vector((side, -0.4, 0.15)).normalized()
            out = (out - d * out.dot(d)).normalized()
            spike(bm_sp, a, a + out * 0.13 + d * 0.04, 0.022)
        # Bands where the dark horn meets the blue.
        i = int(n * 0.55)
        tube(bm_base, p[i] - (p[i + 1] - p[i]) * 0.3, p[i] + (p[i + 1] - p[i]) * 0.3, 0.052, 0.052, 10)
    mesh_obj("horn_base", bm_base)
    mesh_obj("horn_tip", bm_tip)
    mesh_obj("spikes", bm_sp)
    export("horns.obj")


# ---------------------------------------------------------------- cars

def extrude_profile(bm, prof, width):
    """A side profile (list of (y, z), y = forward) extruded across x."""
    left = [bm.verts.new((-width / 2, y, z)) for y, z in prof]
    right = [bm.verts.new((width / 2, y, z)) for y, z in prof]
    n = len(prof)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((left[i], left[j], right[j], right[i]))
    bm.faces.new(list(reversed(left)))
    bm.faces.new(right)


def wheel(bm, x, y, r, w):
    sides = 18
    a = [bm.verts.new((x - w / 2, y + r * math.cos(2 * math.pi * i / sides), r + r * math.sin(2 * math.pi * i / sides))) for i in range(sides)]
    b = [bm.verts.new((x + w / 2, y + r * math.cos(2 * math.pi * i / sides), r + r * math.sin(2 * math.pi * i / sides))) for i in range(sides)]
    for i in range(sides):
        j = (i + 1) % sides
        bm.faces.new((a[i], a[j], b[j], b[i]))
    bm.faces.new(list(reversed(a)))
    bm.faces.new(b)


CARS = {
    # body profile (y forward, z up), cabin profile, width, wheel radius, axle positions, extras
    "super": dict(
        body=[(-2.2, 0.35), (2.25, 0.32), (2.3, 0.55), (1.2, 0.78), (-1.4, 0.95), (-2.2, 0.9)],
        cabin=[(-0.9, 0.9), (1.05, 0.8), (0.1, 1.28), (-0.7, 1.25)],
        width=2.0, r=0.36, axles=(1.45, -1.45), wing=False),
    "muscle": dict(
        body=[(-2.3, 0.38), (2.35, 0.38), (2.35, 0.95), (0.6, 1.0), (-1.6, 1.0), (-2.3, 0.98)],
        cabin=[(-1.3, 0.98), (0.55, 0.98), (0.05, 1.5), (-1.05, 1.5)],
        width=2.0, r=0.4, axles=(1.5, -1.5), wing=False),
    "hyper": dict(
        body=[(-2.3, 0.3), (2.4, 0.28), (2.45, 0.42), (1.4, 0.66), (-1.6, 0.86), (-2.3, 0.8)],
        cabin=[(-0.8, 0.8), (1.25, 0.66), (0.2, 1.14), (-0.55, 1.12)],
        width=2.05, r=0.34, axles=(1.55, -1.5), wing=True),
}


def build_car(kind):
    reset()
    c = CARS[kind]
    bm = bmesh.new()
    extrude_profile(bm, c["body"], c["width"])
    # Side skirts / fenders: a slightly wider band low on the body.
    box(bm, (0, 0, 0.45), (c["width"] + 0.08, 3.2, 0.18))
    if c["wing"]:
        box(bm, (0, -2.05, 1.18), (c["width"] * 0.95, 0.35, 0.06))
        box(bm, (-0.6, -1.95, 1.0), (0.06, 0.2, 0.32))
        box(bm, (0.6, -1.95, 1.0), (0.06, 0.2, 0.32))
    if kind == "muscle":
        box(bm, (0, 1.2, 1.04), (0.6, 1.2, 0.1))  # hood scoop
    mesh_obj("body", bm)
    g = bmesh.new()
    extrude_profile(g, c["cabin"], c["width"] * 0.82)
    mesh_obj("glass", g)
    w = bmesh.new()
    for y in c["axles"]:
        for x in (-c["width"] / 2 + 0.12, c["width"] / 2 - 0.12):
            wheel(w, x, y, c["r"], 0.3)
    mesh_obj("wheels", w)
    l = bmesh.new()
    front = max(p[0] for p in c["body"])
    back = min(p[0] for p in c["body"])
    for x in (-c["width"] / 2 + 0.28, c["width"] / 2 - 0.28):
        box(l, (x, front - 0.02, 0.5), (0.42, 0.06, 0.08))
    mesh_obj("lights", l)
    t = bmesh.new()
    box(t, (0, back + 0.02, 0.72 if kind != "hyper" else 0.62), (c["width"] * 0.85, 0.05, 0.07))
    mesh_obj("tail", t)
    export("car_%s.obj" % kind)


build_horns()
for k in CARS:
    build_car(k)
