-- ORBIS probe (#164, unattended via scripts/dev/octane-run.sh): log the
-- node type ids (octane.NT_*) whose names match a pattern, so a project.ocs
-- variant can create nodes Orbis doesn't export yet (Polygon Side, Ray
-- Switch). Output: ~/Documents/orbis_probe_nodetypes.txt, orbis_probe.done.
local PATTERN = "SIDE|SWITCH"
local docs = (os.getenv("HOME") or "/tmp") .. "/Documents/"
local out = {}
for k, v in pairs(octane) do
    if type(k) == "string" and k:sub(1, 3) == "NT_" then
        for p in PATTERN:gmatch("[^|]+") do
            if k:find(p) then out[#out + 1] = k .. " = " .. tostring(v) end
        end
    end
end
table.sort(out)
local f = io.open(docs .. "orbis_probe_nodetypes.txt", "w")
if f then f:write(table.concat(out, "\n"), "\n"); f:close() end
local d = io.open(docs .. "orbis_probe.done", "w"); if d then d:write(os.date(), "\n"); d:close() end
