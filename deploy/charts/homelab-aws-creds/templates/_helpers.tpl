{{/*
Expand the name of the chart.
*/}}
{{- define "homelab-aws-creds.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "homelab-aws-creds.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "homelab-aws-creds.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Agent Common labels
*/}}
{{- define "homelab-aws-creds.agent.labels" -}}
helm.sh/chart: {{ include "homelab-aws-creds.chart" . }}
{{ include "homelab-aws-creds.agent.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Webhook Common labels
*/}}
{{- define "homelab-aws-creds.webhook.labels" -}}
helm.sh/chart: {{ include "homelab-aws-creds.chart" . }}
{{ include "homelab-aws-creds.webhook.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Agent Selector labels
*/}}
{{- define "homelab-aws-creds.agent.selectorLabels" -}}
app.kubernetes.io/name: {{ include "homelab-aws-creds.name" . }}-agent
app.kubernetes.io/instance: {{ .Release.Name }}-agent
{{- end }}

{{/*
Webhook Selector labels
*/}}
{{- define "homelab-aws-creds.webhook.selectorLabels" -}}
app.kubernetes.io/name: {{ include "homelab-aws-creds.name" . }}-webhook
app.kubernetes.io/instance: {{ .Release.Name }}-webhook
{{- end }}

{{/*
Create the name of the agent service account to use
*/}}
{{- define "homelab-aws-creds.agent.serviceAccountName" -}}
{{- if .Values.agent.serviceAccount.create }}
{{- default (include "homelab-aws-creds.fullname" .) .Values.agent.serviceAccount.name }}-agent
{{- else }}
{{- default "default" .Values.agent.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the webhook service account to use
*/}}
{{- define "homelab-aws-creds.webhook.serviceAccountName" -}}
{{- if .Values.webhook.serviceAccount.create }}
{{- default (include "homelab-aws-creds.fullname" .) .Values.webhook.serviceAccount.name }}-agent
{{- else }}
{{- default "default" .Values.webhook.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the aws roles/service account mapping secret name
*/}}
{{- define "homelab-aws-creds.serviceMapping.secretName" -}}
{{- if .Values.useExistingMappingSecret -}}
{{- .Values.useExistingMappingSecret -}}
{{- else -}}
{{- include "homelab-aws-creds.fullname" . -}}-role-mappings
{{- end }}
{{- end }}

{{/*
Create the name of the cert secret
*/}}
{{- define "homelab-aws-creds.webhook.cert.secretName" -}}
{{- if .Values.webhook.useExistingCertSecret -}}
{{- .Values.webhook.cert.useExistingSecret -}}
{{- else -}}
{{- include "homelab-aws-creds.fullname" . -}}-webhook
{{- end }}
{{- end }}

{{/*
Create the dnsNames for the webhook
*/}}
{{- define "homelab-aws-creds.webhook.cert.dnsNames" -}}
{{- $fullname := include "homelab-aws-creds.fullname" . }}
{{- $namespace := .Release.Namespace }}
{{- $fullname }}
{{ $fullname }}.{{ $namespace }}.svc
{{- if .Values.webhook.enabled }}
{{ $fullname }}-webhook
{{ $fullname }}-webhook.{{ $namespace }}.svc
{{- end }}
{{- end }}
