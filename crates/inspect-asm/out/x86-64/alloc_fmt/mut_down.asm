inspect_asm::alloc_fmt::mut_down:
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
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 64]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	jne .LBB_8
	mov rdi, qword ptr [rsp + 16]
	test rdi, rdi
	je .LBB_2
	mov rsi, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 24]
	lea rax, [rsi + rdx]
	add rdi, rsi
	sub rdi, rdx
	cmp rdi, rax
	jae .LBB_5
	mov rax, rsi
	jmp .LBB_6
.LBB_2:
	mov eax, 1
	xor edx, edx
	jmp .LBB_7
.LBB_5:
	mov rbx, rdx
	mov r15, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, rbx
	mov rax, r15
.LBB_6:
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], rax
.LBB_7:
	add rsp, 112
	pop rbx
	pop r14
	pop r15
	ret
.LBB_8:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]