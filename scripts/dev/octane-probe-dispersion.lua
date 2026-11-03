-- ORBIS probe 112 (#33, unattended via scripts/dev/octane-run.sh <probe> <scene.orbx>): does Universal's dispersion reach the render?
-- Imports Documents/orbis_scene.orbx (Calib-Dispersion), logs the
-- dispersion pins of each Universal material and the kernel, then renders
-- as exported, with Cauchy B 0.05 and with Abbe 3 on the dispersive ones.
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe112.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
local function show(v)
    if type(v) == "table" then local t = {} for k, x in pairs(v) do t[#t+1] = tostring(k).."="..show(x) end return "{"..table.concat(t, ",").."}" end
    return tostring(v)
end
log("start " .. os.date())
local ok, err = pcall(function()
    local root = octane.nodegraph.createRootGraph("orbis import")
    root:importFromFile(docs .. "orbis_scene.orbx")
    local scene = octane.project.getSceneGraph()
    for _, item in ipairs(scene:getOwnedItems()) do item:destroy() end
    scene:copyFrom(root:getOwnedItems())
    root:destroy()
    local rt
    for _, item in ipairs(scene:getOwnedItems()) do
        local p = item:getProperties()
        if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
    end
    local kernel = rt:getInputNode(octane.P_KERNEL)
    log("kernel type " .. kernel:getProperties().type)
    for i = 1, kernel:getPinCount() do
        local n = kernel:getPinInfoIx(i).name
        local c = kernel:getInputNodeIx(i)
        if c and c:getAttributeCount() > 0 then log("  kernel " .. n .. " = " .. show(c:getAttributeIx(1))) end
        if n == "maxsamples" then c:setAttribute(octane.A_VALUE, { 600, 0, 0, 0 }, true) end
    end
    local mats, seen = {}, {}
    local function walk(n)
        if seen[n] then return end
        seen[n] = true
        local p = n:getProperties()
        if p.type == octane.NT_MAT_UNIVERSAL then mats[#mats+1] = n end
        if p.isGraph then for _, x in ipairs(n:getOwnedItems()) do walk(x) end end
        local okc, cnt = pcall(function() return n:getPinCount() end)
        for i = 1, (okc and cnt or 0) do local c = n:getInputNodeIx(i); if c then walk(c) end end
    end
    for _, x in ipairs(scene:getOwnedItems()) do walk(x) end
    log("universal materials " .. #mats)
    local disp = {}
    for _, m in ipairs(mats) do
        local s = m:getProperties().name
        for i = 1, m:getPinCount() do
            local n = m:getPinInfoIx(i).name
            if n == "dispersion_coefficient_B" or n == "dispersionMode" or n == "transmissionType" or n == "ior" or n == "roughness" or n == "hasCaustics" then
                local c = m:getInputNodeIx(i)
                local v = c and c:getAttributeCount() > 0 and show(c:getAttributeIx(1)) or "nil"
                s = s .. " " .. n .. "=" .. v
                if n == "dispersion_coefficient_B" and c then disp[#disp+1] = { m = m, i = i, c = c, v = c:getAttributeIx(1) } end
            end
        end
        log(s)
    end
    local function modepin(m, val)
        for i = 1, m:getPinCount() do
            if m:getPinInfoIx(i).name == "dispersionMode" then
                local c = m:getInputNodeIx(i)
                if c then c:setAttribute(octane.A_VALUE, val, true) else m:setPinValueIx(i, val, true) end
            end
        end
    end
    local function render(name)
        local t = os.time()
        octane.render.start{ renderTargetNode = rt }
        octane.render.saveImage(docs .. "orbis_p112_" .. name .. ".png", octane.imageSaveFormat.PNG_8)
        log(name .. ": " .. (os.time() - t) .. " s")
    end
    render("asis")
    for _, d in ipairs(disp) do
        local v = d.v; local a = type(v) == "table" and v[1] or v
        if a and a > 0 then modepin(d.m, 2); d.c:setAttribute(octane.A_VALUE, { 0.05, 0, 0, 0 }, true) end
    end
    render("cauchy005")
    for _, d in ipairs(disp) do
        local v = d.v; local a = type(v) == "table" and v[1] or v
        if a and a > 0 then modepin(d.m, 1); d.c:setAttribute(octane.A_VALUE, { 3, 0, 0, 0 }, true) end
    end
    render("abbe3")
end)
if not ok then log("ERROR " .. tostring(err)) end
log("done " .. os.date())
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write("done\n"); f:close() end
