inspect_asm::alloc_fmt::try_up_a:
	sub rsp, 120
	mov qword ptr [rsp + 40], rsi
	mov qword ptr [rsp + 48], rdx
	lea rax, [rsp + 40]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 64], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 72], rax
	mov qword ptr [rsp + 80], 2
	mov qword ptr [rsp + 104], 0
	lea rax, [rsp + 56]
	mov qword ptr [rsp + 88], rax
	mov qword ptr [rsp + 96], 1
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB0_1
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	mov rdx, qword ptr [rsp + 16]
	add rdx, rax
	mov rcx, qword ptr [rcx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_0
	mov qword ptr [rcx], rax
.LBB0_0:
	xor eax, eax
	add rsp, 120
	ret
.LBB0_1:
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	mov rsi, qword ptr [rsp + 16]
	mov r8, qword ptr [rsp + 24]
	lea r9, [rax + rsi]
	mov rcx, qword ptr [r8]
	mov rdi, qword ptr [rcx]
	cmp r9, rdi
	je .LBB0_3
	add rsi, rax
	cmp rsi, rdi
	je .LBB0_4
.LBB0_2:
	add rsp, 120
	ret
.LBB0_3:
	lea rsi, [rax + rdx]
	add rsi, 3
	and rsi, -4
	mov qword ptr [rcx], rsi
	mov rcx, qword ptr [r8]
	mov rdi, qword ptr [rcx]
	mov rsi, rdx
	add rsi, rax
	cmp rsi, rdi
	jne .LBB0_2
.LBB0_4:
	lea rsi, [rax + rdx]
	add rsi, 3
	and rsi, -4
	mov qword ptr [rcx], rsi
	add rsp, 120
	ret
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 24]
	mov rsi, qword ptr [rsp + 16]
	add rsi, rcx
	mov rdx, qword ptr [rdx]
	cmp rsi, qword ptr [rdx]
	jne .LBB0_5
	mov qword ptr [rdx], rcx
.LBB0_5:
	mov rdi, rax
	call _Unwind_Resume@PLT
