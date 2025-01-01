```bash
export TARGET_CC=x86_64-linux-musl-gcc
export RUSTFLAGS="-C linker=x86_64-linux-musl-gcc"
IMAGE=utxo-global-tgbot
cargo build --release --target x86_64-unknown-linux-musl
TAG=staging
AWS_ID=604313529175
AWS_ECR_URI=$AWS_ID.dkr.ecr.ap-southeast-1.amazonaws.com
DOCKER_IMAGE=$AWS_ECR_URI/$IMAGE:$TAG
ZONE=ap-southeast-1

docker build --platform=linux/amd64 -f ./k8s/Dockerfile.binary -t $DOCKER_IMAGE .

aws ecr get-login-password --region $ZONE | docker login --username AWS --password-stdin $AWS_ECR_URI
docker push 604313529175.dkr.ecr.ap-southeast-1.amazonaws.com/${IMAGE}:staging

kubectl rollout restart -n utxo-global-$TAG deploy/utxo-global-tgbot
```
