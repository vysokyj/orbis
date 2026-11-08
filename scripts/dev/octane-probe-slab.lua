-- ORBIS probe 71 (unattended: scripts/dev/octane-run.sh <this> <export.orbx>):
-- Calib-SSSThick-* (#86): the slabs (props, exported as Universal) get an
-- OpenPBR (505) random walk with Iray's volume - subsurface 1, base colour
-- 0, no specular, the walk's colour the volume's A(alpha), its radius the
-- mean free path 1 / (sigma_a + sigma_s) - as the skin export would give
-- it (`skin_albedo`, `sss_radius`); then x 0.5 / 2 of the radius. Compare
-- with Iray's render of the same scene: light through a slab of known
-- thickness. VOL (below): "skin" Victoria 8's volume, "grey" SSS Color 0.7.
-- Output: ~/Documents/orbis_p71_mfp_<f>.png, orbis_probe71.txt,
-- orbis_probe.done.
local VOL = "skin"
local VOLUMES = {
    skin = { a = { 0.4643, 0.3474, 0.2451 }, mfp = { 0.002557, 0.000151, 0.000171 } },
    grey = { a = { 0.8146, 0.3435, 0.2856 }, mfp = { 0.000190, 0.000154, 0.000142 } },
}
local SAMPLES = 256
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe71.txt", "w")
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
local function setNode(m, pin, typ, value)
    local ix = pins(m)[pin]
    if not ix then return end
    local c = m:getInputNodeIx(ix)
    if not (c and c:getProperties().type == typ) then
        c = octane.node.create{ type = typ, name = pin, pinOwnerNode = m, pinOwnerIx = ix }
    end
    c:setAttribute(octane.A_VALUE, value, true)
end
try("imager", function()
    local im = rt:getInputNode(octane.P_IMAGER)
    local p = pins(im)
    local function set(pin, v)
        local c = im:getInputNodeIx(p[pin])
        local cur = c:getAttribute(octane.A_VALUE)
        if type(cur) == "table" then cur[1] = v; c:setAttribute(octane.A_VALUE, cur, true) else c:setAttribute(octane.A_VALUE, v, true) end
    end
    local kernel = rt:getInputNode(octane.P_KERNEL)
    kernel:getInputNodeIx(pins(kernel)["maxsamples"]):setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)
    set("response", 403); set("gamma", 1); set("highlightCompression", 0)
    local lut = p["lut"] and im:getInputNodeIx(p["lut"])
    if lut then pcall(function() im:disconnectIx(p["lut"]) end) end
    log("lut left " .. tostring(p["lut"] and im:getInputNodeIx(p["lut"]) ~= nil))
    for _, n in ipairs({ "exposure", "response", "gamma", "acesToneMapping", "white_balance", "saturation" }) do
        if p[n] then pcall(function() log("imager " .. n .. " " .. tostring(im:getInputNodeIx(p[n]):getAttribute(octane.A_VALUE)[1] or im:getInputNodeIx(p[n]):getAttribute(octane.A_VALUE))) end) end
    end
end)
local function render(name)
    local t = os.time()
    octane.render.start{ renderTargetNode = rt }
    log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_p71_" .. name .. ".png", octane.imageSaveFormat.PNG_8)))
end
-- The slabs' materials: whatever Universal / 505 node a slab item's pins
-- hold (the USD import names the mesh items after the prims).
local slabs = {}
try("slabs", function()
    for _, item in ipairs(usd:getOwnedItems()) do
        local name = tostring(item:getProperties().name)
        if name:find("Slab") then
            for ix = 1, item:getPinCount() do
                local info = item:getPinInfoIx(ix)
                local ok, n = pcall(function() return item:getInputNodeIx(ix) end)
                log(name .. " pin " .. ix .. " " .. tostring(info.name) .. " -> " .. tostring(ok and n and n:getProperties().type) .. " " .. tostring(ok and n and n:getProperties().name))
                if ok and n and name:find("^/Scene/Materials/Slab") and ix == 1 then slabs[#slabs + 1] = { item = item, ix = ix } end
            end
        end
    end
    log(#slabs .. " slab material pins")
end)
local v = VOLUMES[VOL]
local max = math.max(v.mfp[1], v.mfp[2], v.mfp[3])
local walks = {}
try("walk", function()
    for _, sl in ipairs(slabs) do
        local m = octane.node.create{ type = 505, name = "slab walk", pinOwnerNode = sl.item, pinOwnerIx = sl.ix }
        setNode(m, "base", octane.NT_TEX_FLOAT, 1)
        setNode(m, "baseColor", octane.NT_TEX_RGB, { 0, 0, 0 })
        setNode(m, "specular", octane.NT_TEX_FLOAT, 0)
        setNode(m, "subsurface", octane.NT_TEX_FLOAT, 1)
        setNode(m, "subsurfaceColor", octane.NT_TEX_RGB, v.a)
        setNode(m, "scale", octane.NT_TEX_RGB, { v.mfp[1] / max, v.mfp[2] / max, v.mfp[3] / max })
        setNode(m, "subsurfaceAnisotropy", octane.NT_FLOAT, { 0, 0, 0, 0 })
        walks[#walks + 1] = m
    end
end)
for _, f in ipairs({ 1, 0.5, 2 }) do
    try("mfp" .. f, function()
        for _, m in ipairs(walks) do setNode(m, "radius", octane.NT_FLOAT, { max * f, 0, 0, 0 }) end
        render(string.format("mfp_%.1f", f))
    end)
end
pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
