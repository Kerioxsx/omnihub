# Vesper Client models, built in Blender (run headless):
#   blender -b --python client/models/vesper_models.py -- <out_dir> [render_dir]
# Writes wing.obj, crown.obj, skiff.obj (units: blocks; OBJ +Y up, +Z toward
# the player's front) into <out_dir>, and optional preview renders.
# Every shape is generated here from code: original designs, no imported art.

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


def tube(bm, a, b, r0, r1, sides=6):
    """A tapered tube from a to b (Blender coordinates)."""
    a, b = Vector(a), Vector(b)
    d = (b - a).normalized()
    up = Vector((0, 0, 1)) if abs(d.z) < 0.9 else Vector((1, 0, 0))
    u = d.cross(up).normalized()
    v = d.cross(u).normalized()
    ring_a, ring_b = [], []
    for i in range(sides):
        t = 2 * math.pi * i / sides
        off = u * math.cos(t) + v * math.sin(t)
        ring_a.append(bm.verts.new(a + off * r0))
        ring_b.append(bm.verts.new(b + off * r1))
    for i in range(sides):
        j = (i + 1) % sides
        bm.faces.new((ring_a[i], ring_a[j], ring_b[j], ring_b[i]))
    bm.faces.new(list(reversed(ring_a)))
    bm.faces.new(ring_b)


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


# ---------------------------------------------------------------- wing
# A right wing, pivot at the shoulder (origin). Blender: +X outward, +Z up,
# -Y backward (behind the player). Arm bone up and out, four finger bones
# fanning down and back; the membrane spans between them with a scalloped
# trailing edge and a slight sag, like a bat or dragon wing.
def build_wing():
    reset()
    elbow = Vector((0.42, -0.10, 0.30))
    wrist = Vector((0.80, -0.16, 0.52))
    fingers = [
        Vector((1.30, -0.30, 0.78)),   # leading tip, high
        Vector((1.42, -0.36, 0.22)),
        Vector((1.18, -0.34, -0.30)),
        Vector((0.78, -0.26, -0.62)),
    ]
    root_low = Vector((0.10, -0.06, -0.30))  # where the membrane meets the back

    bones = bmesh.new()
    tube(bones, (0, 0, 0), elbow, 0.045, 0.035)
    tube(bones, elbow, wrist, 0.035, 0.028)
    for f in fingers:
        tube(bones, wrist, f, 0.024, 0.008)
    claws = bmesh.new()
    spike(claws, wrist, wrist + Vector((0.02, 0.0, 0.16)), 0.03)
    spike(claws, elbow, elbow + Vector((0.0, 0.04, 0.10)), 0.022)

    mem = bmesh.new()
    # Edge chain: shoulder -> elbow -> wrist -> fingers... -> root_low, then fan panels.
    def panel(p0, p1, p2, sag, steps=6):
        """Curved triangle p0 (wrist side), p1, p2 with a scalloped edge between p1 and p2."""
        rows = []
        for i in range(steps + 1):
            t = i / steps
            edge = p1.lerp(p2, t)
            mid = (p1 + p2) / 2
            # Scallop: pull the trailing edge toward p0 between the bones.
            scallop = math.sin(math.pi * t) * 0.22
            edge = edge.lerp(p0, scallop)
            rows.append(edge)
        verts_edge = []
        verts_mid = []
        for i, e in enumerate(rows):
            t = i / steps
            m = p0.lerp(e, 0.5)
            m.y -= sag * math.sin(math.pi * t) + sag * 0.5
            verts_mid.append(mem.verts.new(m))
            verts_edge.append(mem.verts.new(e))
        c = mem.verts.new(p0)
        for i in range(steps):
            mem.faces.new((c, verts_mid[i], verts_mid[i + 1]))
            mem.faces.new((verts_mid[i], verts_edge[i], verts_edge[i + 1], verts_mid[i + 1]))
    chain = fingers + [root_low]
    for i in range(len(chain) - 1):
        panel(wrist, chain[i], chain[i + 1], 0.06)
    # Inner panel: shoulder, elbow, wrist down to root_low.
    a, b, c2, d = mem.verts.new((0, 0, 0)), mem.verts.new(elbow), mem.verts.new(wrist), mem.verts.new(root_low)
    mem.faces.new((a, b, c2))
    mem.faces.new((a, c2, d))

    # Glow tips (small diamonds at each finger end) for the bloom pass.
    tips = bmesh.new()
    for f in fingers:
        s = 0.035
        vs = [tips.verts.new(f + Vector(o)) for o in ((s, 0, 0), (0, 0, s), (-s, 0, 0), (0, 0, -s))]
        tips.faces.new(vs)

    mesh_obj("membrane", mem)
    mesh_obj("bone", bones)
    mesh_obj("claw", claws)
    mesh_obj("tip", tips)
    export("wing.obj")
    if RENDER:
        preview("wing", ((0.9, 0.2, 0.1), "membrane"), ((0.12, 0.05, 0.05), "bone"), ((0.9, 0.9, 0.9), "claw"), ((1, 0.7, 0.5), "tip"), cam=(2.6, -3.2, 1.0), look=(0.7, -0.2, 0.1))


# ---------------------------------------------------------------- crown
# A thorn crown: a twisted band with 9 curved thorns of two lengths.
def build_crown():
    reset()
    band = bmesh.new()
    n = 36
    r = 0.22
    ring_top, ring_bot = [], []
    for i in range(n):
        t = 2 * math.pi * i / n
        w = 0.02 * math.sin(3 * t)
        ring_top.append(band.verts.new((math.cos(t) * r, math.sin(t) * r, 0.025 + w)))
        ring_bot.append(band.verts.new((math.cos(t) * (r + 0.015), math.sin(t) * (r + 0.015), -0.025 + w)))
    for i in range(n):
        j = (i + 1) % n
        band.faces.new((ring_top[i], ring_top[j], ring_bot[j], ring_bot[i]))
    thorns = bmesh.new()
    for i in range(9):
        t = 2 * math.pi * i / 9
        L = 0.20 if i % 2 == 0 else 0.12
        base = Vector((math.cos(t) * r, math.sin(t) * r, 0.0))
        out = Vector((math.cos(t), math.sin(t), 0.0))
        tip = base + out * L * 0.35 + Vector((0, 0, L))
        spike(thorns, base, tip, 0.028)
    mesh_obj("band", band)
    mesh_obj("thorn", thorns)
    export("crown.obj")


# ---------------------------------------------------------------- skiff
# A compact hover-cart prop for the showroom: rounded hull, cockpit glass,
# tail fins, two rear thrusters and an underglow strip. Purely decorative.
def build_skiff():
    reset()
    bpy.ops.mesh.primitive_cube_add(size=1)
    hull = bpy.context.object
    hull.name = "hull"
    hull.scale = (1.1, 2.1, 0.42)
    hull.location = (0, 0, 0.55)
    bpy.ops.object.transform_apply(scale=True, location=True)
    bev = hull.modifiers.new("bev", "BEVEL")
    bev.width = 0.18
    bev.segments = 3
    bpy.ops.object.modifier_apply(modifier="bev")
    # Nose taper.
    for v in hull.data.vertices:
        if v.co.y < -0.6:
            k = (-0.6 - v.co.y) / 0.45
            v.co.x *= 1 - 0.35 * k
            v.co.z = 0.55 + (v.co.z - 0.55) * (1 - 0.4 * k)

    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=8, radius=1)
    glass = bpy.context.object
    glass.name = "glass"
    glass.scale = (0.42, 0.62, 0.30)
    glass.location = (0, -0.15, 0.78)
    bpy.ops.object.transform_apply(scale=True, location=True)

    fins = bmesh.new()
    for sx in (-1, 1):
        a = fins.verts.new((sx * 0.45, 0.85, 0.75))
        b = fins.verts.new((sx * 0.62, 1.15, 1.25))
        c = fins.verts.new((sx * 0.52, 1.10, 0.75))
        fins.faces.new((a, b, c))
    mesh_obj("fin", fins)

    thr = bmesh.new()
    for sx in (-0.35, 0.35):
        tube(thr, (sx, 0.95, 0.5), (sx, 1.18, 0.5), 0.13, 0.15, sides=10)
    mesh_obj("thruster", thr)

    glow = bmesh.new()
    pts = [(-0.5, -0.8), (0.5, -0.8), (0.55, 0.9), (-0.55, 0.9)]
    vs = [glow.verts.new((x, y, 0.32)) for x, y in pts]
    glow.faces.new(vs)
    mesh_obj("glow", glow)
    export("skiff.obj")
    if RENDER:
        preview("skiff", ((0.08, 0.09, 0.12), "hull"), ((0.4, 0.75, 1.0), "glass"), ((0.15, 0.15, 0.2), "fin"), ((0.3, 0.3, 0.35), "thruster"), ((0.2, 0.8, 1.0), "glow"), cam=(3.4, -3.6, 2.2), look=(0, 0, 0.6))


def export(name):
    for ob in bpy.context.scene.objects:
        ob.select_set(ob.type == "MESH")
    bpy.ops.wm.obj_export(filepath=os.path.join(OUT, name), export_selected_objects=True, export_materials=False,
                          export_uv=False, export_normals=False, export_triangulated_mesh=True,
                          forward_axis="Z", up_axis="Y")
    print("wrote", name)


def preview(name, *parts, cam=(3, -3, 2), look=(0, 0, 0)):
    """A quick Cycles render for the design sheet (not used by the mod)."""
    scene = bpy.context.scene
    for col, part in parts:
        ob = bpy.data.objects.get(part)
        if not ob:
            continue
        m = bpy.data.materials.new(part)
        m.use_nodes = True
        bsdf = m.node_tree.nodes["Principled BSDF"]
        bsdf.inputs["Base Color"].default_value = (*col, 1)
        if part in ("tip", "glow", "glass"):
            bsdf.inputs["Emission Color"].default_value = (*col, 1)
            bsdf.inputs["Emission Strength"].default_value = 4.0
        ob.data.materials.append(m)
    world = bpy.data.worlds.new("w")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.02, 0.025, 0.05, 1)
    scene.world = world
    bpy.ops.object.light_add(type="AREA", location=(2, -2, 4))
    bpy.context.object.data.energy = 600
    bpy.ops.object.camera_add(location=cam)
    c = bpy.context.object
    d = Vector(look) - Vector(cam)
    c.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    scene.camera = c
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.device = "CPU"
    scene.cycles.use_denoising = False
    scene.cycles.samples = 48
    scene.render.resolution_x = 900
    scene.render.resolution_y = 600
    scene.render.filepath = os.path.join(RENDER, name + ".png")
    bpy.ops.render.render(write_still=True)


build_wing()
build_crown()
build_skiff()
