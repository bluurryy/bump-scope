inspect_asm::alloc_fmt::try_mut_down_a:
	push r15
	push r14
	push rbx
	sub rsp, 112
	mov qword ptr [rsp + 32], rsi
	mov qword ptr [rsp + 40], rdx
	lea rax, [rsp + 32]
	mov qword ptr [rsp + 48], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 64], rax
	mov qword ptr [rsp + 72], 2
	mov qword ptr [rsp + 96], 0
	lea rax, [rsp + 48]
	mov qword ptr [rsp + 80], rax
	mov qword ptr [rsp + 88], 1
	mov qword ptr [rsp], 1
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 8], xmm0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_1]
	mov rdi, rsp
	lea rdx, [rsp + 64]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB0_0
	xor eax, eax
	jmp .LBB0_4
.LBB0_0:
	mov rdi, qword ptr [rsp + 16]
	test rdi, rdi
	je .LBB0_1
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 24]
	lea rax, [rsi + rbx]
	add rdi, rsi
	sub rdi, rbx
	cmp rdi, rax
	jae .LBB0_2
	mov rax, rsi
	jmp .LBB0_3
.LBB0_1:
	mov eax, 1
	xor ebx, ebx
	jmp .LBB0_4
.LBB0_2:
	mov rdx, rbx
	mov r15, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r15
.LBB0_3:
	mov rcx, qword ptr [r14]
	mov rdx, rax
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB0_4:
	mov rdx, rbx
	add rsp, 112
	pop rbx
	pop r14
	pop r15
	ret
