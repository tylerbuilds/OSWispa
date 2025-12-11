#!/bin/bash
# scripts/setup_gnome_shortcut.sh
# Adds a custom shortcut for OSWispa to GNOME settings

NAME="OSWispa Toggle"
COMMAND="/usr/local/bin/oswispa-toggle"
BINDING="<Control><Super>"

# Check if already exists
EXISTING=$(/usr/bin/gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings)
if [[ $EXISTING == *"$COMMAND"* ]]; then
    echo "Snapshot already exists or command is already bound."
    exit 0
fi

# Base path for custom keybindings
SCHEMA="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding"
KEY_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"

# Find a free slot
for i in {0..99}; do
    SLOT="custom$i"
    PATH="$KEY_PATH/$SLOT/"
    
    # Check if this slot is already in the list
    if [[ $EXISTING != *"$PATH"* ]]; then
        echo "Found free slot: $SLOT"
        
        # Set the custom keybinding properties
        /usr/bin/gsettings set "$SCHEMA:$PATH" name "$NAME"
        /usr/bin/gsettings set "$SCHEMA:$PATH" command "$COMMAND"
        /usr/bin/gsettings set "$SCHEMA:$PATH" binding "$BINDING"
        
        # Add to the list (handling empty list case correctly)
        if [ "$EXISTING" == "@as []" ]; then
            NEW_LIST="['$PATH']"
        else
            # Remove closing bracket and append
            NEW_LIST="${EXISTING%]*}, '$PATH']"
        fi
        
        /usr/bin/gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$NEW_LIST"
        echo "Shortcut set successfully: $NAME ($BINDING) -> $COMMAND"
        exit 0
    fi
done

echo "Error: specific slot finding failed or too many custom shortcuts."
exit 1
