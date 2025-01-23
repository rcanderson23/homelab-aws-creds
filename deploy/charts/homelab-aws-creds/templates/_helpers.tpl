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
Common labels
*/}}
{{- define "homelab-aws-creds.labels" -}}
helm.sh/chart: {{ include "homelab-aws-creds.chart" . }}
{{ include "homelab-aws-creds.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "homelab-aws-creds.selectorLabels" -}}
app.kubernetes.io/name: {{ include "homelab-aws-creds.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "homelab-aws-creds.agent.serviceAccountName" -}}
{{- if .Values.agent.serviceAccount.create }}
{{- default (include "homelab-aws-creds.fullname" .) .Values.agent.serviceAccount.name }}-agent
{{- else }}
{{- default "default" .Values.agent.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the aws roles/service account mapping secret name
*/}}
{{- define "homelab-aws-creds.serviceMapping.secretName" -}}
{{- if .Values.useExistingMappingSecret -}}
{{- .Values.useExistingMappingSecret -}}
{{- else -}}
{{- default (include "homelab-aws-creds.fullname" .) .Values.agent.serviceAccount.name }}-agent
{{- include "homelab-aws-creds.fullname" . -}}-role-mappings
{{- end -}}
{{- end -}}
