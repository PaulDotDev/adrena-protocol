#!/bin/bash

# adrena_anchor_build - Build Anchor project and trim enums to a single variant

echo "Building Adrena with Anchor..."
anchor build || { echo "Anchor build failed"; exit 1; }

echo "Cleaning IDL..."
IDL_PATH="./target/idl/adrena.json"
TS_PATH="./target/types/adrena.ts"

# Target enums to trim
ENUM_PATTERN="Title|Wallpaper|ProfilePicture|Achievement"

# Function to trim enum variants to just one
trim_enums_file() {
  FILE="$1"
  perl -0777 -pe "
    # Match and reduce target enums to a single variant
    s/(\"name\"\\s*:\\s*\"($ENUM_PATTERN)\".+?\"variants\"\\s*:\\s*\\[)[^\\]]+?(\\])/\\1 { \"name\": \"Zero\" } \\3/gs;

    # Fix common structural issues
    s/,\\s*(\\]|\\})/\\1/g;
    s/\\{\\s*,/\\{/g;
    s/,\\s*\\}/\\}/g;
  " "$FILE" > "${FILE}.tmp" && mv "${FILE}.tmp" "$FILE"
}

# Process both files
for FILE in "$IDL_PATH" "$TS_PATH"; do
  echo "Trimming enums in $(basename "$FILE")..."
  trim_enums_file "$FILE"
done

# Add metadata to JSON IDL
echo "Validating and adding metadata to IDL..."
if jq empty "$IDL_PATH" 2>/dev/null; then
  jq '. + {"metadata": {"address": "13gDzEXCdocbj8iAiqrScGo47NiSuYENGsRqi3SEAwet"}}' "$IDL_PATH" > "${IDL_PATH}.tmp" && mv "${IDL_PATH}.tmp" "$IDL_PATH"
  echo "✅ Done! Enums trimmed to a single variant, IDL metadata injected."
else
  echo "❌ Error: $IDL_PATH is invalid JSON. Check for trailing commas or broken structure."
  exit 1
fi
