( WIP ) 
  
tPanel - telemetry panel
---

IDEA:  
Simple system that can do three things:
- collect metrics ( eg. os, i2c sensors, modbus devies, etc)and send to mqtt / api (sender)
- receive sensor data from mqtt, store it (core) and show it (panel).
---
                ┌──────────────────────────────────────────┐
                │  				  pub/sub 				   │
                └──────────────────────────────────────────┘
                     ▲				 │				│
                     │   			 │				│
                     │				 ▼				▼
                ┌─────────┐     ┌─────────┐		┌─────────┐
                │  sender │		│  core	  │	──►	│  panel  │
                └─────────┘		└─────────┘		└─────────┘
                	 ▲
                	 │
                	 │
                ┌─────────┐                       
                │  reader │                       
                └─────────┘  
---
build: 
```
cargo build --workspace --release
```

run:
```
#panel: new terminal
cd panel && cargo run --release
```

```
#sender: new terminal; 
cd sender && cargo build --release
 ../target/release/generator | cargo run --release
```

watch:
```
http://localhost:3000/c3983603-3404-4d43-9234-85bbc8fea3191/
```

test:
```
cargo test -p sender
cargo test -p reader
```
