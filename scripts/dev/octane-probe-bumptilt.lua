-- ORBIS probe (#75, unattended via scripts/dev/octane-run.sh <this> <scene.orbx>):
-- how far Octane's bump tilts the normal. The Info channel kernel renders
-- the shading normal (type 1) without and with "Bump and normal mapping",
-- unfiltered (it reads the base layer's `bump`, not `coatingBump`: move the
-- height maps there in the exported project.ocs first).
-- The tilt per pixel is the angle between the two (scripts/dev/bump-tilt.py).
-- Output: ~/Documents/orbis_bt_flat.png, orbis_bt_bump.png (8-bit),
-- orbis_probe_bumptilt.txt, orbis_probe.done.

local SAMPLES = 16
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe_bumptilt.txt", "w")
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

local kernel
try("kernel", function()
    local rp = pins(rt)
    kernel = octane.node.create{ type = octane.NT_KERN_INFO, name = "info", pinOwnerNode = rt, pinOwnerIx = rp["kernel"] }
    local kp = pins(kernel)
    kernel:getInputNodeIx(kp["type"]):setAttribute(octane.A_VALUE, 1, true)
    kernel:getInputNodeIx(kp["maxsamples"]):setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)
    kernel:getInputNodeIx(kp["samplingMode"]):setAttribute(octane.A_VALUE, 2, true)
end)

local function render(name, bump)
    try(name, function()
        kernel:getInputNodeIx(pins(kernel)["bump"]):setAttribute(octane.A_VALUE, bump, true)
        local t = os.time()
        octane.render.start{ renderTargetNode = rt }
        local fmt = octane.imageSaveFormat.PNG_16 or octane.imageSaveFormat.PNG_8
        log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_bt_" .. name .. ".png", fmt)))
    end)
end
render("flat", false)
render("bump", true)

pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
