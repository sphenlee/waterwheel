Kubernetes Waterwheel Setup
===========================

This folder contains the files needed to get a basic Waterwheel setup 
running in Kubernetes.

This example will use `minikube`.

1. First build a Docker image and load into the local Docker host:

   ```
   bazel build //:waterwheel_load
   ```

1. Start minikube:

   ```
   minikube start
   ```

1. Enable the `ingress` and `ingress-dns` addons:

   ```
   minikube addons enable ingress
   minikube addons enable ingress-dns
   ```

1. Follow the instructions from Minikube to configure DNS resolutions
   for `*.kube` domain.

   For example on Ubuntu using systemd-resolved:

   ```bash
   IFACE="$(ip -json route get "$(minikube ip)" | jq -r .[0].dev)"
   resolvectl dns "$IFACE" "$(minikube ip)"
   resolvectl domain "$IFACE" ~kube
   ```

1. Generate the required RSA keypair.
   This will require `openssl` installed.

   ```bash
   cd etc/kube/
   just gen-keypair
   ```

1. Load the image into minikube

   ```
   minikube image load waterwheel:local
   ```


1. Apply the kubernetes descriptors:

   ``` 
   just apply
   ```

1. check the web interface at https://waterwheel.kube/ . You will need to 
   login as username `fry`  and password `fry`. Other members of the Planet Express crew
   also work if you want to test authorization.

1. Create the sample jobs

   ```bash
   cd sample/
   eval $(minikube docker-env)
   export WATERWHEEL_ADDR=https://waterwheel.kube/
   just deploy
   ```
