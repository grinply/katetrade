# Makefile for trading visualization

# Variables
IMAGE_NAME = trading-visualization
OUTPUT_DIR = output
CANDLESTICK_CSV = candlestick.csv
TRADES_CSV = trades.csv
OUTPUT_HTML = output/trade-results.html

# Default target
.PHONY: run
run: $(OUTPUT_HTML)
	@echo "Visualization generated at $(OUTPUT_HTML)"

.PHONY: open
open:
	xdg-open output/trade-results.html

# Build the Docker image
.PHONY: build
build:
	docker build -t $(IMAGE_NAME) .

# Run the Docker container
$(OUTPUT_HTML): $(CANDLESTICK_CSV) $(TRADES_CSV) | $(OUTPUT_DIR)
	docker run --rm \
		-v $(PWD)/$(CANDLESTICK_CSV):/app/$(CANDLESTICK_CSV):ro \
		-v $(PWD)/$(TRADES_CSV):/app/$(TRADES_CSV):ro \
		-v $(PWD)/$(OUTPUT_DIR):/app/$(OUTPUT_DIR) \
		$(IMAGE_NAME) /app/$(CANDLESTICK_CSV) /app/$(TRADES_CSV) /app/$(OUTPUT_HTML)


# Create output directory if it doesn't exist
$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

# Clean up output
.PHONY: clean
clean:
	rm -rf $(OUTPUT_DIR)
	@echo "Output directory cleaned."

# Rebuild the Docker image
.PHONY: rebuild
rebuild: clean build

# Help message
.PHONY: help
help:
	@echo "Makefile for trading visualization"
	@echo
	@echo "Usage:"
	@echo "  make build      - Build the Docker image"
	@echo "  make run        - Run the Docker container and generate visualization"
	@echo "  make clean      - Remove the output directory"
	@echo "  make rebuild    - Clean and rebuild the Docker image"
	@echo "  make help       - Show this help message"
