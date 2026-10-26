-- ORBIS batch render (run by scripts/dev/octane-render.sh through
-- octane-run.sh): every line of Documents/orbis_jobs.txt is
-- "<file.orbx>\t<name>\t<samples>"; each export is opened, rendered as
-- exported (its own imager, tone LUT included) and saved as
-- Documents/orbis_job_<name>.png.

local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe_render.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
log("start " .. os.date())
for line in io.lines(docs .. "orbis_jobs.txt") do
    local file, name, samples = line:match("^([^\t]+)\t([^\t]+)\t(%d+)")
    local ok, err = pcall(function()
        assert(octane.project.load(docs .. file), "load failed")
        local rt
        for _, item in ipairs(octane.project.getSceneGraph():getOwnedItems()) do
            local p = item:getProperties()
            if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
        end
        -- A project opened from Lua leaves the imager's tone LUT without
        -- its package and file (opened from File > Open it has them, probe
        -- 96); pointing it back into the package didn't load it either. The
        -- runner extracts the package's tone.cube beside it: load that by
        -- its absolute path, as probe 93 did.
        local im = rt:getInputNode(octane.P_IMAGER)
        local cube = docs .. file:gsub("%.orbx$", ".cube")
        local fh = io.open(cube, "r")
        if fh then
            fh:close()
            for i = 1, im:getPinCount() do
                if im:getPinInfoIx(i).name == "lut" then
                    local lut = im:getInputNodeIx(i)
                    if lut then
                        lut:setAttribute(octane.A_PACKAGE, "", true)
                        lut:setAttribute(octane.A_FILENAME, cube, true)
                        pcall(function() lut:evaluate() end)
                        log(name .. ": tone LUT from " .. cube)
                    end
                end
            end
        end
        local kernel = rt:getInputNode(octane.P_KERNEL)
        for i = 1, kernel:getPinCount() do
            if kernel:getPinInfoIx(i).name == "maxsamples" then
                kernel:getInputNodeIx(i):setAttribute(octane.A_VALUE, { tonumber(samples), 0, 0, 0 }, true)
            end
        end
        local t = os.time()
        octane.render.start{ renderTargetNode = rt }
        local saved = octane.render.saveImage(docs .. "orbis_job_" .. name .. ".png", octane.imageSaveFormat.PNG_8)
        log(name .. ": " .. (os.time() - t) .. " s, saved " .. tostring(saved))
        octane.project.reset()
    end)
    if not ok then log(name .. ": ERROR " .. tostring(err)) end
end
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
