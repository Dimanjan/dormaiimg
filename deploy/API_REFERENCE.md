# Dharke Clothing Matcher — Headless API Reference & Deployment Guide

A self-contained, ultra-low resource REST API for clothing variant matching and product authenticity verification.

- **Standing Server Memory**: **~15 MB RAM**
- **Binary Footprint**: **~1.1 MB** (100% self-contained single file with embedded reference models)
- **Runtime Dependencies**: **Zero** (no Python, no OpenCV, no external assets needed at runtime)
- **Response Latency**: **~50 – 70 ms**

---

## 1. Hosting Options

### Option A: Direct Single-Binary Deployment (Simplest for any Linux VPS)

1. Copy the compiled executable to your server (e.g., using `scp`):
   ```bash
   scp rust_matcher/target/release/rust_matcher user@your-server-ip:/opt/rust_matcher
   ```

2. Run it on your desired port:
   ```bash
   chmod +x /opt/rust_matcher
   /opt/rust_matcher 8000
   ```
   *Note: You can also pass the port via environment variable: `PORT=8000 /opt/rust_matcher`.*

---

### Option B: Systemd Service (Ubuntu / Debian / CentOS VPS with Auto-Restart)

1. Copy the executable to `/usr/local/bin`:
   ```bash
   sudo cp rust_matcher/target/release/rust_matcher /usr/local/bin/clothing-matcher
   sudo chmod +x /usr/local/bin/clothing-matcher
   ```

2. Create `/etc/systemd/system/clothing-matcher.service`:
   ```ini
   [Unit]
   Description=Dharke Clothing Matcher API
   After=network.target

   [Service]
   Type=simple
   User=www-data
   Group=www-data
   Environment=PORT=8000
   ExecStart=/usr/local/bin/clothing-matcher
   Restart=always
   RestartSec=3
   LimitNOFILE=65536
   MemoryMax=64M

   [Install]
   WantedBy=multi-user.target
   ```

3. Enable and start:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now clothing-matcher
   ```

4. Check status and logs:
   ```bash
   sudo systemctl status clothing-matcher
   journalctl -u clothing-matcher -f
   ```

---

### Option C: Docker Container

1. Build container image:
   ```bash
   docker build -t clothing-matcher:latest .
   ```

2. Run container (with 64MB memory limit):
   ```bash
   docker run -d --name matcher -p 8000:8000 --memory="64m" clothing-matcher:latest
   ```

---

## 2. API Endpoints

### 1. Evaluate Image: `POST /evaluate` (or `/api/evaluate`)

Evaluates an uploaded customer image or mobile screenshot and returns both Task 1 (Color Matching) and Task 2 (Product Verification) in a structured JSON payload.

#### Supported Request Formats
The endpoint automatically detects and accepts:
- **Direct Raw Binary**: `Content-Type: image/jpeg`, `image/png`, `image/webp`, or `application/octet-stream`
- **Multipart Form-Data**: `Content-Type: multipart/form-data; boundary=...` with file in `file` or `image` field

---

#### Example 1: cURL (Direct Raw Binary — Recommended)
```bash
curl -X POST http://your-server:8000/evaluate \
  -H "Content-Type: image/jpeg" \
  --data-binary @customer_photo.jpg
```

#### Example 2: cURL (Multipart Form-Data)
```bash
curl -X POST http://your-server:8000/evaluate \
  -F "file=@customer_photo.jpg"
```

#### Example 3: Python (`requests`)
```python
import requests

# Direct binary
with open("customer_photo.jpg", "rb") as f:
    response = requests.post(
        "http://your-server:8000/evaluate",
        data=f.read(),
        headers={"Content-Type": "image/jpeg"}
    )

print(response.json())
```

#### Example 4: Node.js (`fetch`)
```javascript
import fs from "fs";

const imageBuffer = fs.readFileSync("customer_photo.jpg");

const response = await fetch("http://your-server:8000/evaluate", {
  method: "POST",
  headers: { "Content-Type": "image/jpeg" },
  body: imageBuffer,
});

const result = await response.json();
console.log(result);
```

#### Example 5: PHP (`curl`)
```php
<?php
$ch = curl_init('http://your-server:8000/evaluate');
$fileData = file_get_contents('customer_photo.jpg');

curl_setopt($ch, CURLOPT_POST, true);
curl_setopt($ch, CURLOPT_POSTFIELDS, $fileData);
curl_setopt($ch, CURLOPT_HTTPHEADER, ['Content-Type: image/jpeg']);
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);

$response = curl_exec($ch);
curl_close($ch);

$data = json_decode($response, true);
print_r($data);
?>
```

---

### JSON Response Format

```json
{
  "success": true,
  "color_evaluation": {
    "detected_color": "Dharke Grey",
    "matched_variant_id": "dharke_grey",
    "confidence": 98.1,
    "grey_score": 98.1,
    "black_score": 1.9,
    "estimated_l_star": 71.9,
    "estimated_chroma": 1.4,
    "is_neutral": true,
    "delta_e_grey": 0.1,
    "delta_e_black": 33.9,
    "dominant_swatches": [
      {
        "hex_code": "#B1B0AE",
        "percentage": 45.0,
        "name": "Highlight / Fabric Weave",
        "l_star": 71.9,
        "a_star": -0.0,
        "b_star": 1.1
      },
      {
        "hex_code": "#585657",
        "percentage": 35.0,
        "name": "Base Fabric",
        "l_star": 36.8,
        "a_star": 1.0,
        "b_star": -0.3
      },
      {
        "hex_code": "#474747",
        "percentage": 20.0,
        "name": "Shadow / Rib Ridge",
        "l_star": 30.2,
        "a_star": 0.0,
        "b_star": 0.0
      }
    ],
    "explanation": "Fabric lightness L*=71.9 matches Dharke Grey (expected ~72.0, Delta-E 0.1). 20.8% fabric pixels have high lightness."
  },
  "product_verification": {
    "is_genuine_product": true,
    "verification_status": "VERIFIED_GENUINE",
    "confidence": 95.5,
    "inlier_count": 29,
    "good_matches_count": 29,
    "matched_template": "dharke_grey",
    "bounding_box": null,
    "visualization_base64": null,
    "explanation": "Strong feature match confirmed (29 pattern keypoints). Distinctive Dharke weave is verified."
  },
  "query_metadata": {
    "filename": "image",
    "width": 1080,
    "height": 2400,
    "aspect_ratio": "1080:2400"
  }
}
```

---

### Response Fields Breakdown

#### `color_evaluation` (Task 1)
- `detected_color`: `"Dharke Grey"`, `"Dharke Black"`, or `"Other / Saturated Color"`.
- `matched_variant_id`: `"dharke_grey"`, `"dharke_black"`, or `null`.
- `confidence`: Confidence rating (65% – 99%).
- `grey_score` / `black_score`: Probability distribution between Grey and Black.
- `estimated_l_star`: Fabric lightness on CIELAB $L^*$ scale (0 – 100).
- `is_neutral`: `true` for neutral greyscale/black; `false` if high chromatic saturation is detected.
- `dominant_swatches`: 3 extracted hex swatches representing fabric highlights, base body, and weave shadows.

#### `product_verification` (Task 2)
- `is_genuine_product`: `true` if matched against genuine Dharke templates; `false` if customer sent a different product.
- `verification_status`: `"VERIFIED_GENUINE"` ($\ge 20$ inliers), `"LIKELY_GENUINE"` ($14\text{--}19$ inliers), or `"DIFFERENT_PRODUCT"` ($< 14$ inliers).
- `confidence`: Verification confidence percentage.
- `inlier_count`: Number of matched pattern feature descriptors.
- `matched_template`: `"dharke_grey"` or `"dharke_black"`.

---

### 2. Health Check: `GET /health`
```bash
curl http://your-server:8000/health
```
**Response**:
```json
{
  "status": "ok",
  "service": "clothing-matcher-api",
  "version": "1.0.0",
  "memory_profile": "ultra-low (~15MB)"
}
```

---

### 3. Base Truth Catalogue: `GET /ground-truth`
```bash
curl http://your-server:8000/ground-truth
```
Returns metadata about the embedded reference templates.
