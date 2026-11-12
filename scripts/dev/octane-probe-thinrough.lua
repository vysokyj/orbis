-- ORBIS probe (#136, unattended via scripts/dev/octane-run.sh):
-- Calib-MetalRefr.orbx (enhanced-final). Iray blurs the view through a
-- thin-walled rough refraction (DAZ's `refraction_base` transmits through
-- `simple_glossy_bsdf` at the glossy roughness, thin wall or not);
-- Universal's Thin Wall transmission ignores roughness (manual; orbx
-- variants twr5 / s1 / s1r5 confirmed it, and Specular at IOR 1 doesn't
-- blur either). Does any other Octane material blur a thin wall?
-- R1 / R2 (refraction 0.3 / 0.7, white, glossy roughness 0.2 -> Beckmann
-- pin 0.2) replaced by: the Specular material with Thin wall on, at pin
-- 0.2 and 0.5; OpenPBR Surface thin-walled; Standard Surface thin-walled
-- (with its roughnessExtra). All render the edge behind R2 sharp.
-- `bg` first makes R1 / R2 / X4 a clear thin wall (the background a
-- straight ray sees, for the blur model in the lab note); X4 stays so.
-- Pins are found by name (logged with every node's full pin list).
-- Output: ~/Documents/orbis_p136_*.png, orbis_probe136.txt, orbis_probe.done.

local SAMPLES = 400
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe136.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
local function try(label, fn)
    local ok, err = pcall(fn)
    if not ok then log("ERROR " .. label .. ": " .. tostring(err)) end
    return ok
end
local function pins(node)
    local t = {}
    for i = 1, node:getPinCount() do t[node:getPinInfoIx(i).name] = i end
    return t
end

log("start " .. os.date())
local rt, usd
try("load", function()
    log("load: " .. tostring(octane.project.load(docs .. "orbis_scene.orbx")))
    for _, item in ipairs(octane.project.getSceneGraph():getOwnedItems()) do
        local p = item:getProperties()
        if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
        if p.name and p.name:find("%(DAZ 3D%)") then usd = item end
    end
end)
try("samples", function()
    local kernel = rt:getInputNode(octane.P_KERNEL)
    kernel:getInputNodeIx(pins(kernel)["maxsamples"]):setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)
end)

-- The material inputs (20007) of R1 / R2 and their refraction weight.
local slots = {}
try("slots", function()
    for _, item in ipairs(usd:getOwnedItems()) do
        local name = item:getProperties().name or ""
        if name:find("/R1_refraction") then slots[#slots + 1] = { item = item, w = 0.3 } end
        if name:find("/R2_refraction") then slots[#slots + 1] = { item = item, w = 0.7 } end
        if name:find("/X4_metal") then slots[#slots + 1] = { item = item, w = 0.7, x4 = true } end
    end
    log(#slots .. " slots")
end)

-- Set a pin by value on its own (default or new) input node.
local function setValue(node, ix, value)
    local c = node:getInputNodeIx(ix)
    local info = node:getPinInfoIx(ix)
    if not c then
        local typ = ({ [octane.PT_BOOL] = octane.NT_BOOL, [octane.PT_FLOAT] = octane.NT_FLOAT, [octane.PT_ENUM] = octane.NT_ENUM,
                       [octane.PT_INT] = octane.NT_INT })[info.type]
        if not typ then typ = type(value) == "table" and octane.NT_TEX_RGB or octane.NT_TEX_FLOAT end
        c = octane.node.create{ type = typ, name = info.name, pinOwnerNode = node, pinOwnerIx = ix }
    end
    local ok = pcall(function() c:setAttribute(octane.A_VALUE, value, true) end)
    if not ok and type(value) == "number" then
        ok = pcall(function() c:setAttribute(octane.A_VALUE, { value, 0, 0 }, true) end)
    end
    if not ok and type(value) == "table" then
        ok = pcall(function() c:setAttribute(octane.A_VALUE, value[1], true) end)
    end
    return ok
end

-- rules: list of { match(lowercase pin name) -> bool, value(w) }.
local function material(typ, label, rules, w)
    return function(slot)
        local m = octane.node.create{ type = typ, name = label, pinOwnerNode = slot.item, pinOwnerIx = 1 }
        local names = {}
        for i = 1, m:getPinCount() do
            local info = m:getPinInfoIx(i)
            local n = info.name
            names[#names + 1] = n .. ":" .. tostring(info.type)
            for _, r in ipairs(rules) do
                if r[1](n:lower()) then
                    local v = r[2](slot.w)
                    log(string.format("  %s.%s <- %s: %s", label, n, type(v) == "table" and table.concat(v, " ") or tostring(v), tostring(setValue(m, i, v))))
                    break
                end
            end
        end
        if not slot.logged then log("  pins " .. label .. ": " .. table.concat(names, ", ")) end
    end
end
local function eq(s) return function(n) return n == s end end
local function has(...)
    local parts = { ... }
    return function(n)
        for _, p in ipairs(parts) do if not n:find(p, 1, true) then return false end end
        return true
    end
end
local function k(v) return function() return v end end
local grey = function(w) return { w, w, w } end

local function specular(rough)
    return material(octane.NT_MAT_SPECULAR, "spec", {
        { eq("reflection"), k({ 1, 1, 1 }) }, { eq("transmission"), grey }, { eq("brdf"), k(1) },
        { eq("roughness"), k(rough) }, { eq("ior"), k(1.5) }, { has("thin"), k(true) },
        { eq("fake_shadows"), k(true) },
    })
end
local base = { 0.051783, 0.000412, 0.000252 }
local openpbr = material(octane.NT_MAT_OPENPBR or 505, "openpbr", {
    { eq("transmission"), function(w) return w end }, { eq("transmissioncolor"), k({ 1, 1, 1 }) },
    { eq("roughness"), k(0.5) }, { eq("ior"), k(1.5) }, { eq("thinwall"), k(true) },
    { eq("basecolor"), k(base) }, { eq("fake_shadows"), k(true) },
})
local standard = material(octane.NT_MAT_STANDARD_SURFACE or 277, "std", {
    { eq("transmission"), function(w) return w end }, { eq("transmissioncolor"), k({ 1, 1, 1 }) },
    { eq("roughness"), k(0.5) }, { eq("roughnessextra"), k(0.5) }, { eq("ior"), k(1.5) }, { eq("thinwall"), k(true) },
    { eq("basecolor"), k(base) }, { eq("base"), k(1) }, { eq("fake_shadows"), k(true) },
})

-- The background as a straight-through view sees it (for a blur model of
-- X4's deficit): a clear thin wall, no reflection, no base.
local clear = material(octane.NT_MAT_UNIVERSAL, "clear", {
    { eq("transmission"), k({ 1, 1, 1 }) }, { eq("transmissiontype"), k(2) }, { eq("specular"), k({ 0, 0, 0 }) },
    { eq("albedo"), k({ 0, 0, 0 }) }, { eq("ior"), k(1) }, { eq("fake_shadows"), k(true) },
})

local function render(name)
    local t = os.time()
    octane.render.start{ renderTargetNode = rt }
    log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_p136_" .. name .. ".png", octane.imageSaveFormat.PNG_8)))
end
local function variant(name, make)
    try(name, function()
        for _, s in ipairs(slots) do make(s); s.logged = true end
        for _, s in ipairs(slots) do s.logged = nil end
        render(name)
    end)
end

-- Run 1: spec_tw, spec_tw_r5 (Specular, pin 0.2 / 0.5; its IOR then the
-- default), OpenPBR / Standard Surface with pins misnamed (no transmission
-- weight or roughness set): sharp. Run 2, the pins by their real names:
local function only(pred, make) return function(slot) if pred(slot) then make(slot) end end end
local notx4 = function(s) return not s.x4 end
variant("bg", clear)
variant("spec_tw_r5_ior", only(notx4, specular(0.5)))
variant("openpbr_tw_r5", only(notx4, openpbr))
variant("std_tw_r5_extra", only(notx4, standard))

pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
