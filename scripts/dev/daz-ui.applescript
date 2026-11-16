-- ORBIS: read or press one control of the queue-side DAZ Studio's main
-- window through System Events (the terminal needs Accessibility access).
--   osascript scripts/dev/daz-ui.applescript value|enabled|click <AXRole> <name>
-- The first element of that role whose name (or, unnamed, description) is
-- <name>, searched depth-first through the window's tree. Prints its value / enabled state, or "clicked"; "missing" when no
-- such element is in the window (e.g. the Render Settings pane is hidden).
-- Used by iray-queue-start.sh (#194).
on run argv
	set act to item 1 of argv
	set wantRole to item 2 of argv
	set wantName to item 3 of argv
	tell application "System Events" to tell process "DAZStudio"
		if (count of windows) is 0 then return "missing"
		set e to my find(window 1, wantRole, wantName, 0)
		if e is missing value then return "missing"
		if act is "click" then
			click e
			return "clicked"
		else if act is "enabled" then
			return (enabled of e) as text
		else
			return (value of e) as text
		end if
	end tell
end run

-- Depth-first through UI elements: "entire contents" came back empty for
-- the same window on some calls.
on find(el, wantRole, wantName, depth)
	if depth > 25 then return missing value
	tell application "System Events"
		set kids to {}
		try
			set kids to UI elements of el
		end try
		repeat with k in kids
			try
				if role of k is wantRole then
					set n to ""
					try
						set n to name of k
					end try
					if n is missing value or n is "" then
						try
							set n to description of k
						end try
					end if
					if n is wantName then return contents of k
				end if
			end try
			set r to my find(k, wantRole, wantName, depth + 1)
			if r is not missing value then return r
		end repeat
	end tell
	return missing value
end find
