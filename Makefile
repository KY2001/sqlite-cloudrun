GIT_SHA := $(shell git rev-parse HEAD 2>/dev/null)

# Cloud Run
GCP_REGION ?= asia-northeast1
AR_REPO := $(GCP_REGION)-docker.pkg.dev/$(SERVER_PROJECT_ID)/$(SERVER_NAME)
IMAGE := $(AR_REPO)/$(SERVER_NAME)

# Task
.PHONY: run
run:
	mkdir -p data
	DB_PATH=data/app.db PORT=$(SERVER_PORT) cargo run

.PHONY: gen
gen:
	rm -rf openapi/src
	docker run --rm --user $(shell id -u):$(shell id -g) --volume $(CURDIR):/local \
		openapitools/openapi-generator-cli:v7.16.0 generate -i /local/openapi/openapi.yaml -g rust-axum \
		-o /local/openapi --additional-properties=packageName=openapi,hideGenerationTimestamp=true

.PHONY: build
build:
	cargo build --release --locked

.PHONY: format
format:
	cargo fmt

.PHONY: lint
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	docker run --rm -i hadolint/hadolint < Dockerfile

.PHONY: lint-fix
lint-fix: format
	cargo clippy --fix --allow-dirty --allow-staged

.PHONY: all
all: gen lint-fix lint build

.PHONY: upgrade-packages
upgrade-packages:
	cargo update

.PHONY: docker-build-and-run
docker-build-and-run: docker-build docker-run

.PHONY: docker-build
docker-build:
	docker build . --platform linux/amd64 --tag $(SERVER_NAME) --load --progress=plain

.PHONY: docker-run
docker-run:
	docker container run --rm --interactive --tty --user $(shell id -u) --tmpfs /data:mode=1777 \
		--env GCS_BUCKET=$(GCS_BUCKET) --env GOOGLE_APPLICATION_CREDENTIALS=/adc.json \
		--volume $(HOME)/.config/gcloud/application_default_credentials.json:/adc.json:ro \
		-p $(SERVER_PORT):8080 $(SERVER_NAME)

.PHONY: gcloud-ar-login
gcloud-ar-login:
	gcloud auth configure-docker $(GCP_REGION)-docker.pkg.dev --quiet

.PHONY: gcloud-build-and-push
gcloud-build-and-push:
	docker build . --platform linux/amd64 --tag $(IMAGE):$(SERVER_ENV)-$(GIT_SHA) --progress plain
	docker tag $(IMAGE):$(SERVER_ENV)-$(GIT_SHA) $(IMAGE):$(SERVER_ENV)-latest
	docker push $(IMAGE):$(SERVER_ENV)-$(GIT_SHA)
	docker push $(IMAGE):$(SERVER_ENV)-latest

.PHONY: gcloud-deploy
gcloud-deploy:
	gcloud run deploy $(SERVER_NAME) --project=$(SERVER_PROJECT_ID) --region=$(GCP_REGION) \
		--image=$(IMAGE):$(SERVER_ENV)-latest \
		--service-account=$(SERVER_NAME)@$(SERVER_PROJECT_ID).iam.gserviceaccount.com \
		--set-env-vars=GCS_BUCKET=$(GCS_BUCKET) \
		--max=1 --max-instances=1 --min=0 --concurrency=80 --cpu=1 --memory=512Mi \
		--timeout=60 --cpu-throttling
